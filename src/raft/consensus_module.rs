use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Notify, broadcast};
use tokio::task::JoinSet;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use std::sync;
use log;
use crate::raft::append_entry_type::{AppendEntryArgs, AppendEntryReply};
use super::types::*;
use super::request_voted_type::*;
pub struct ConsensusModule {
    lock: Arc<Mutex<()>>,

    id: i64,
    peer_ids: Vec<i64>,

    current_term: i64,
    voted_for: i64,
    log: Vec<LogEntry>,

    commit_index: i64,
    last_applied: i64,
    state: CMState,
    election_reset_event: DateTime<Utc>,

    next_index: HashMap<i64, i64>,
    match_index: HashMap<i64, i64>,

    // 1) “chan<- CommitEntry” => mpsc::Sender<CommitEntry>
    //   Client giữ Receiver; CM giữ Sender và .send().await
    commit_chan: mpsc::Sender<CommitEntry>,

    // 2) & 4) “chan struct{}” => Notify (tín hiệu không mang dữ liệu)
    //   Notify hợp với kiểu “đánh thức khi có thay đổi”, tránh nghẽn do đầy buffer
    new_commit_ready_chan: Arc<Notify>,
    trigger_ae_chan: Arc<Notify>,

    // 3) WaitGroup => JoinSet hoặc Vec<JoinHandle<()>> để join khi shutdown
    new_commit_ready_chan_wg: JoinSet<()>,

    tasks: Mutex<JoinSet<()>>
    // (tùy chọn) phát tín hiệu shutdown cho tất cả task
    // shutdown_tx: broadcast::Sender<()>,
}

impl ConsensusModule {
    pub fn new(id: i64, peer_ids: Vec<i64>, commit_chan: mpsc::Sender<CommitEntry>) -> arc::muxSelf {
        ConsensusModule {
            lock: Arc::new(Mutex::new(())),
            id,
            peer_ids,
            current_term: 0,
            voted_for: -1,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            state: CMState::Follower,
            election_reset_event: Utc::now(),
            next_index: HashMap::new(),
            match_index: HashMap::new(),
            commit_chan,
            new_commit_ready_chan: Arc::new(Notify::new()),
            trigger_ae_chan: Arc::new(Notify::new()),
            new_commit_ready_chan_wg: JoinSet::new(),
            tasks: Mutex::new(JoinSet::new()),
        }
    }

    pub fn start(self: &Arc<Self>, ready_rx: tokio::sync::oneshot::Receiver<()>) -> () {
        {
            let me = Arc::clone(&self);
            self.tasks.lock().unwrap().spawn(async move {
                let _ = ready_rx.await;

                {
                    let _g = me.lock.lock().unwrap();
                    me.election_reset_event = Utc::now();
                }

                me.run_election_timer().await;
            });
        }

        {
            let me = Arc::clone(self);
            self.tasks.lock().unwrap().spawn(async move {
                me.commit_chan_sender().await;
            });
        }
    }

    pub fn request_voted(&mut self, args: &RequestVotedArgs, reply: &mut RequestVotedReply) -> () {
        // let lock = self.lock.clone();
        let mut _lock = self.lock.lock().unwrap();

        reply.vote_granted = false;
        reply.term = self.current_term;

        if self.state == CMState::Dead {
            return;
        }

        let (local_last_log_index, local_last_log_term) = self.get_last_log_index_and_term();
        log::info!("RequestVote: [current_term: {}, voted_for: {}, log index/term: ({}, {})]",
            self.current_term, self.voted_for, local_last_log_index, local_last_log_term);

        if args.term > self.current_term {
            log::info!("... term out of date in RequestVote");
            self.become_follower(args.term);
        }
        reply.term = self.current_term;

        let is_upto_date = (args.term > self.current_term) ||
            (args.term == self.current_term && args.last_log_index >= local_last_log_index);

        if !is_upto_date{
            log::info!("... candidate log not up-to-date [candidate:{}/{} < local:{}/{}] -> reject",
			    args.last_log_term, args.last_log_index, local_last_log_term, local_last_log_index);
            return;
        }

        if self.voted_for == -1 || self.voted_for == args.candidate_id {
            self.voted_for = args.candidate_id;
            reply.vote_granted = true;

            log::info!("... grant vote to {}", args.candidate_id);
        } else{
            log::info!("... already voted for {} in term {} -> reject", self.voted_for, self.current_term);
        }
    }

    pub fn append_entries(&mut self, args: &AppendEntryArgs, reply: &mut AppendEntryReply){

        reply.success = false;
        reply.term = self.current_term;

        if self.state == CMState::Dead {
            return;
        }
        log::info!("AppendEntries: [currentTerm={}, commitIndex={}, logLen={}]",
            self.current_term, self.commit_index, self.log.len());

        // 1) Term check
        if args.term > self.current_term {
            log::info!("... term out of date in AppendEntries");
            return;
        }
        if args.term > self.current_term || self.state != CMState::Follower {
            log::info!("... step down to follower, update term to {}", args.term);
            self.become_follower(args.term);
        }
        reply.term = self.current_term;

        // 2) Reset election timer
        self.election_reset_event = Utc::now();

        // 3) Check PrevLogIndex/PrevLogTerm
        // if args.prev_log_index >= 0 {
        //     if args.prev_log_index >= self.log.len() as i64 {
        //         log::info!("... reject: PrevLogIndex {} out of range (len={})",
        //             args.prev_log_index, self.log.len() as i64);
        //         return;
        //     }
        //     if self.log[args.prev_log_index as usize].term != args.prev_log_term {
        //         log::info!("... reject: term mismatch at idx={} (local={}, prev={})",
        //             args.prev_log_index, self.log[args.prev_log_index as usize].term, args.prev_log_term);
        //         return;
        //     }
        // }

        if args.prev_log_index == -1 ||
            (args.prev_log_index < self.log.len() as i64 && args.prev_log_term == self.log[args.prev_log_index as usize].term) {
            reply.success = true;

            // Find an insertion point - where there's a term mismatch between
            // the existing log starting at PrevLogIndex+1 and the new entries sent
            // in the RPC.
            let mut log_insert_index = args.prev_log_index + 1;
            let mut new_entries_index = 0;

            loop {
                if log_insert_index >= self.log.len() as i64 || new_entries_index >= args.entries.len() {
                    break;
                }
                if self.log[log_insert_index as usize].term != args.entries[new_entries_index].term {
                    break;
                }

                log_insert_index += 1;
                new_entries_index += 1;
            }
            // At the end of this loop:
            // - logInsertIndex points at the end of the log, or an index where the
            //   term mismatches with an entry from the leader
            // - newEntriesIndex points at the end of Entries, or an index where the
            //   term mismatches with the corresponding log entry
            if new_entries_index < args.entries.len() {
                log::info!("... inserting entries from index {}", log_insert_index);
                self.log.truncate(log_insert_index as usize);
                self.log.extend_from_slice(&args.entries[new_entries_index..]);
                log::info!("... log is now: {:?}", self.log)
            }

            // Set commit index.
            if args.leader_commit > self.commit_index {
                self.commit_index = std::cmp::min(args.leader_commit, self.log.len() as i64 - 1);
                log::info!("... setting commitIndex={}", self.commit_index)
            }
        } else {
            // No match for PrevLogIndex/PrevLogTerm. Populate
            // ConflictIndex/ConflictTerm to help the leader bring us up to date
            // quickly.
            if args.prev_log_index >= self.log.len() as i64 {
                reply.conflict_index = self.log.len() as i64;
                reply.conflict_term = -1
            } else {
                // PrevLogIndex points within our log, but PrevLogTerm doesn't match
                // cm.log[PrevLogIndex].
                reply.conflict_term = self.log[args.prev_log_index as usize].term;

                let mut i = args.prev_log_index - 1;
                while i >= 0 {
                    if self.log[i as usize].term != reply.conflict_term {
                        break;
                    }
                    i -= 1;
                }
                reply.conflict_index = (i + 1);
            }
        }
        reply.term = self.current_term;
        log::info!("AppendEntries reply: {:?}", reply)
    }

    pub fn leader_send_append_entries(&mut self){
        // Lock
        if self.state != CMState::Leader{
            return
        }
        let saved_current_term = self.current_term;
        // Unlock

        let peer_ids: Vec<i64> = self.peer_ids.clone();

        for peer_id in &peer_ids{
        // new thread
            // lock
            let next_idx = self.next_index.get(&peer_id).cloned().unwrap();
            let prev_log_index = next_idx - 1;
            let prev_log_term = if prev_log_index < 0 { -1 } else {self.log[prev_log_index as usize].term};
            let entries = self.log[prev_log_index as usize..].to_vec();
            let entries_len = entries.len() as i64;

            let args = AppendEntryArgs{
                term: saved_current_term,
                leader_id: self.id,
                prev_log_index,
                prev_log_term,
                entries,
                leader_commit: self.commit_index
            };
            // unlock
            log::info!("sending AppendEntries to {}: ni={}, args={:?}", peer_id, next_idx, args);

            // Just example
            let reply = AppendEntryReply{
                term: 0,
                success: true,
                conflict_term: 0,
                conflict_index: 0
            };
            // Call server to send args
            // Happy path
            // lock
            if reply.term > self.current_term{
                log::warn!("term out of date in heartbeat reply");
                self.become_follower(reply.term);
                return
            }

            if self.state == CMState::Leader && saved_current_term == reply.term {
                if reply.success{
                    self.next_index.insert(*peer_id, next_idx + entries_len);
                    self.match_index.insert(*peer_id, self.next_index.get(&peer_id).unwrap() - 1);

                    let saved_commit_index = self.commit_index;

                    for i in (self.commit_index + 1)..self.log.len() as i64{
                        if self.log[i as usize].term == self.current_term {
                            let mut match_count = 1;
                            for peer_id in &peer_ids {
                                if let Some(&val) = self.match_index.get(peer_id) {
                                    if val >= i {
                                        match_count += 1;
                                    }
                                }
                                if match_count*2 >= peer_ids.len() + 1{
                                    self.commit_index = i;
                                }
                            }

                            log::info!("AppendEntries reply from {} success: commitIndex := {}",
                                *peer_id, self.commit_index);
                            if self.commit_index > saved_commit_index{
                                log::info!("leader sets commitIndex := {}", self.commit_index);

                                //lock
                                // cm.newCommitReadyChan <- struct{}{}
                                // cm.triggerAEChan <- struct{}{}
                            }
                            else{
                                // unlock
                            }
                        } else{
                            if reply.conflict_term >= 0 {
                                let mut last_index_of_term = -1;
                                for i in (0..self.log.len() as i64).rev() {
                                    if self.log[i as usize].term == reply.conflict_term {
                                        last_index_of_term = i;
                                        break;
                                    }
                                }
                                if last_index_of_term >= 0 {
                                    //TODO: Not sure last_index_of_term + 1 or last_index_of_term
                                    self.next_index.insert(*peer_id, last_index_of_term + 1);
                                } else {
                                    self.next_index.insert(*peer_id, reply.conflict_index);
                                }
                            } else {
                                self.next_index.insert(*peer_id, reply.conflict_index);
                            }
                            log::info!("AppendEntries reply from {} conflict: nextIndex := {}",
                                *peer_id, next_idx - 1);
                            // unlock
                        }
                    }
                }
            }
            // unlock
        }
    }
}

impl ConsensusModule {
    fn get_last_log_index_and_term(&self) -> (i64, i64) {
        if self.log.len() > 0 {
            let last_index = self.log.len() - 1;
            (last_index as i64, self.log[last_index].term)
        } else {
            (0, 0)
        }
    }

    fn become_follower(&mut self, term: i64) {
        log::info!("Become Follower with term: {}", term);
        self.state = CMState::Follower;
        self.current_term = term;
        self.voted_for = -1;

    }

    async fn run_election_timer(self){

    }

    async fn commit_chan_sender(self){

    }
}