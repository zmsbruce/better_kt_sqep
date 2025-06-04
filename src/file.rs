use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        mpsc::{Sender, channel},
    },
    thread,
    time::Duration,
};

use crate::{
    error::{Error, GraphError},
    graph::{AddonEntityType, DistinctEntityType, KnowledgeGraph, Relation, Snapshot},
};

static FILE_WRITE_LOCK: Mutex<()> = Mutex::new(());

pub struct FiledKnowledgeGraph {
    graph: KnowledgeGraph,
    pub file_path: PathBuf,
    save_sender: Sender<Snapshot>,
}

impl FiledKnowledgeGraph {
    pub fn new<P>(path: P, create: bool) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        // 如果文件不存在，则创建一个空文件
        let graph = if !path.as_ref().exists() || create {
            fs::write(path.as_ref(), "")?;

            // 创建一个空的知识图谱
            KnowledgeGraph::from_snapshot(Snapshot::default())
        } else {
            // 读取文件到字符串
            let file_content = fs::read_to_string(path.as_ref())?;

            // 解析字符串到知识图谱
            let snapshot = Snapshot::from_xml(&file_content)?;
            KnowledgeGraph::from_snapshot(snapshot)
        };

        let file_path = path.as_ref().to_path_buf();
        // 创建保存通知通道
        let (tx, rx) = channel::<Snapshot>();

        // 启动保存线程（可根据需要调整线程退出策略，此处为永久运行）
        let save_file_path = file_path.clone();
        thread::spawn(move || {
            // 线程循环等待保存通知
            while let Ok(snapshot) = rx.recv() {
                // 等待一段时间，收集短时间内的其它通知
                thread::sleep(Duration::from_millis(50));
                let mut latest_snapshot = snapshot;
                // drain所有当前通道中剩余的快照，取最后一个
                while let Ok(new_snapshot) = rx.try_recv() {
                    latest_snapshot = new_snapshot;
                }
                // 使用最新的快照进行保存
                match latest_snapshot.to_xml() {
                    Ok(xml) => {
                        // 获取文件写锁
                        let _lock = match FILE_WRITE_LOCK.lock() {
                            Ok(lock) => lock,
                            Err(e) => {
                                eprintln!("获取文件写锁失败: {}", e);
                                continue;
                            }
                        };
                        // 写入文件
                        if let Err(e) = fs::write(&save_file_path, xml) {
                            eprintln!("自动保存失败: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("序列号失败: {}", e);
                    }
                }
            }
        });

        Ok(Self {
            graph,
            file_path,
            save_sender: tx,
        })
    }

    pub fn save(&self) -> Result<(), Error> {
        let xml = self.graph.current.to_xml()?;
        let _lock = match FILE_WRITE_LOCK.lock() {
            Ok(lock) => lock,
            Err(e) => return Err(Error::Poison(e.to_string())),
        };
        fs::write(&self.file_path, xml).map_err(Error::Io)
    }

    /// 在修改图谱后调用此方法，将当前快照发送给保存线程以触发保存操作
    fn notify_save(&self) {
        // 发送当前快照（克隆一份数据，避免后续修改影响保存）
        let snapshot = self.graph.current_snapshot().clone();
        // 如果发送失败，则说明保存线程可能已退出，此处打印错误
        if let Err(e) = self.save_sender.send(snapshot) {
            eprintln!("发送保存通知失败: {}", e);
        }
    }

    // 以下方法包装了 KnowledgeGraph 的修改接口，
    // 并在成功修改后调用 notify_save() 自动保存

    pub fn add_entity(
        &mut self,
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
        coor: (f64, f64),
    ) -> u64 {
        let id = self
            .graph
            .add_entity(content, distinct_type, addon_types, coor);
        self.notify_save();
        id
    }

    pub fn remove_entity(&mut self, id: u64) -> Result<(), GraphError> {
        let res = self.graph.remove_entity(id);
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    pub fn update_entity_content(
        &mut self,
        id: u64,
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
    ) -> Result<(), GraphError> {
        let res = self
            .graph
            .update_entity_content(id, content, distinct_type, addon_types);
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    pub fn update_entity_position(
        &mut self,
        id: u64,
        new_pos: (f64, f64),
    ) -> Result<(), GraphError> {
        let res = self.graph.update_entity_position(id, new_pos);
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    pub fn add_edge(&mut self, from: u64, to: u64, relation: Relation) -> Result<(), GraphError> {
        let res = self.graph.add_edge(from, to, relation);
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    pub fn remove_edge(&mut self, from: u64, to: u64) -> Result<(), GraphError> {
        let res = self.graph.remove_edge(from, to);
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    pub fn update_edge(
        &mut self,
        from: u64,
        to: u64,
        relation: Relation,
    ) -> Result<(), GraphError> {
        let res = self.graph.update_edge(from, to, relation);
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    pub fn undo(&mut self) -> Result<(), GraphError> {
        let res = self.graph.undo();
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    pub fn redo(&mut self) -> Result<(), GraphError> {
        let res = self.graph.redo();
        if res.is_ok() {
            self.notify_save();
        }
        res
    }

    #[inline]
    pub fn current_snapshot(&self) -> &Snapshot {
        self.graph.current_snapshot()
    }
}
