//! 教学知识图谱模块，提供了一个支持撤回和重做操作的知识图谱数据结构。
//!
//! # 支持
//! - 只支持教学知识图谱，不支持能力知识图谱；
//! - 节点不支持资源型独立实体类型；

use im::{HashMap, Vector};

use crate::error::GraphError;
pub use node::{AddonEntityType, DistinctEntityType, EntityNode, Relation};

mod codec;
mod node;

/// 知识图谱快照，用于撤回和重做。
/// 使用了 im crate 提供的持久化数据结构，避免了不必要的数据复制，提高了性能。
/// 详见：https://docs.rs/im/15.0.0/im/
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub nodes: HashMap<u64, EntityNode>,
    pub edges: HashMap<(u64, u64), Relation>,
    latest_id: u64,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            latest_id: 1, // 从 1 开始避免兼容问题
        }
    }
}

/// 教学知识图谱，支持撤回和重做操作。
#[derive(Debug)]
pub struct KnowledgeGraph {
    pub current: Snapshot,
    undo_stack: Vector<Snapshot>,
    redo_stack: Vector<Snapshot>,
    max_history: usize,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self {
            current: Snapshot::default(),
            undo_stack: Vector::new(),
            redo_stack: Vector::new(),
            max_history: 100,
        }
    }
}

impl KnowledgeGraph {
    /// 从快照创建一个新的图谱
    pub fn from_snapshot(snapshot: Snapshot) -> Self {
        Self {
            current: snapshot,
            ..Default::default()
        }
    }

    /// 执行修改前的公共操作。
    /// 1. 清空重做栈
    /// 2. 如果历史记录超过最大值，删除最早的记录
    /// 3. 将当前快照压入撤回栈
    fn before_mutation(&mut self) {
        // 清空重做栈
        self.redo_stack.clear();

        // 如果历史记录超过最大值，删除最早的记录
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }

        // 将当前快照压入撤回栈
        self.undo_stack.push_back(self.current.clone());
    }

    /// 撤回上一次操作。
    /// 如果没有操作可撤回，返回错误。
    pub fn undo(&mut self) -> Result<(), GraphError> {
        // 从撤回栈中取出上一个快照。如果没有快照，返回错误。
        let current = self
            .undo_stack
            .pop_back()
            .ok_or(GraphError::NothingToUndo)?;

        // 将当前快照压入重做栈
        self.redo_stack.push_back(self.current.clone());

        // 将上一个快照设置为当前快照
        self.current = current;

        Ok(())
    }

    /// 重做上一次操作。
    /// 如果没有操作可重做，返回错误。
    pub fn redo(&mut self) -> Result<(), GraphError> {
        // 从重做栈中取出上一个快照。如果没有快照，返回错误。
        let current = self
            .redo_stack
            .pop_back()
            .ok_or(GraphError::NothingToRedo)?;

        // 将当前快照压入撤回栈
        self.undo_stack.push_back(self.current.clone());

        // 将上一个快照设置为当前快照
        self.current = current;

        Ok(())
    }

    /// 添加一个节点
    pub fn add_entity(
        &mut self,
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
        coor: (f64, f64),
    ) -> u64 {
        self.before_mutation(); // 记录快照

        // 生成新节点 ID
        let current = &mut self.current;
        let id = current.latest_id;
        current.latest_id += 1;

        // 插入新节点
        current.nodes.insert(
            id,
            EntityNode::new(id, content, distinct_type, addon_types, coor),
        );

        id
    }

    /// 删除一个节点及其关联的边
    /// 如果节点不存在，返回错误。
    pub fn remove_entity(&mut self, id: u64) -> Result<(), GraphError> {
        self.before_mutation(); // 记录快照

        // 删除节点，如果节点不存在则返回错误
        let current = &mut self.current;
        if current.nodes.remove(&id).is_none() {
            return Err(GraphError::EntityNotFound(id));
        }

        // 删除关联的边
        current
            .edges
            .retain(|(from, to), _| *from != id && *to != id);

        Ok(())
    }

    /// 修改节点内容
    /// 如果节点不存在，返回错误。
    pub fn update_entity_content(
        &mut self,
        id: u64,
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
    ) -> Result<(), GraphError> {
        self.before_mutation(); // 记录快照

        // 修改节点内容，如果节点不存在则返回错误
        self.current
            .nodes
            .get_mut(&id)
            .map_or(Err(GraphError::EntityNotFound(id)), |node| {
                node.update(content, distinct_type, addon_types, node.coor);

                Ok(())
            })
    }

    /// 修改节点位置，delta 为位置增量。
    /// 如果节点不存在，返回错误。
    pub fn update_entity_position(
        &mut self,
        id: u64,
        new_pos: (f64, f64),
    ) -> Result<(), GraphError> {
        self.before_mutation(); // 记录快照

        // 修改节点位置，如果节点不存在则返回错误
        self.current
            .nodes
            .get_mut(&id)
            .map_or(Err(GraphError::EntityNotFound(id)), |node| {
                node.coor = new_pos;

                Ok(())
            })
    }

    /// 添加一条边。
    /// 如果节点 ID 不存在，或边已经存在，返回错误。
    pub fn add_edge(&mut self, from: u64, to: u64, relation: Relation) -> Result<(), GraphError> {
        self.before_mutation(); // 记录快照

        // 检查节点是否存在
        let current = &mut self.current;
        if !current.nodes.contains_key(&from) {
            return Err(GraphError::EntityNotFound(from));
        }
        if !current.nodes.contains_key(&to) {
            return Err(GraphError::EntityNotFound(to));
        }

        current.edges.insert((from, to), relation);

        Ok(())
    }

    /// 删除一条边
    /// 如果边不存在，返回错误。
    pub fn remove_edge(&mut self, from: u64, to: u64) -> Result<(), GraphError> {
        self.before_mutation(); // 记录快照

        // 删除边，如果边不存在则返回错误
        if self.current.edges.remove(&(from, to)).is_none() {
            return Err(GraphError::EdgeNotFound(from, to));
        }

        Ok(())
    }

    /// 修改边关系
    pub fn update_edge(
        &mut self,
        from: u64,
        to: u64,
        relation: Relation,
    ) -> Result<(), GraphError> {
        self.before_mutation(); // 记录快照

        self.current.edges.get_mut(&(from, to)).map_or(
            Err(GraphError::EdgeNotFound(from, to)),
            |edge| {
                *edge = relation;

                Ok(())
            },
        )
    }

    /// 获取当前快照
    #[inline]
    pub fn current_snapshot(&self) -> &Snapshot {
        &self.current
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn default_distinct() -> DistinctEntityType {
        DistinctEntityType::KnowledgePoint
    }

    fn default_addons() -> Vec<AddonEntityType> {
        vec![
            AddonEntityType::Knowledge,
            AddonEntityType::Thinking,
            AddonEntityType::Example,
            AddonEntityType::Question,
            AddonEntityType::Question, // 重复的类型
        ]
    }

    fn default_relation() -> Relation {
        Relation::Contain
    }

    fn default_coor() -> (f64, f64) {
        (0.0, 0.0)
    }

    #[test]
    fn test_add_entity() {
        let mut graph = KnowledgeGraph::default();
        let content = "Test Node".to_string();
        let id = graph.add_entity(
            content.clone(),
            default_distinct(),
            &default_addons(),
            default_coor(),
        );

        // 检查节点是否存在
        assert!(graph.current.nodes.contains_key(&id));

        // 检查节点
        let node = graph.current.nodes.get(&id).unwrap();
        assert_eq!(node.content, content);
        assert_eq!(node.distinct_type, default_distinct());
        assert_eq!(node.addon_types.len(), 4); // 重复的类型被去除
    }

    #[test]
    fn test_remove_entity() {
        let mut graph = KnowledgeGraph::default();
        let id = graph.add_entity(
            "Node to remove".to_string(),
            default_distinct(),
            &default_addons(),
            default_coor(),
        );
        // 检查节点是否存在
        assert!(graph.current.nodes.contains_key(&id));

        // 删除节点
        assert!(graph.remove_entity(id).is_ok());
        assert!(!graph.current.nodes.contains_key(&id));

        // 再次删除相同节点应该失败
        match graph.remove_entity(id) {
            Err(GraphError::EntityNotFound(eid)) => assert_eq!(eid, id),
            _ => panic!("Expected EntityNotFound error"),
        }
    }

    #[test]
    fn test_update_entity() {
        let mut graph = KnowledgeGraph::default();
        let id = graph.add_entity(
            "Old Content".to_string(),
            default_distinct(),
            &default_addons(),
            default_coor(),
        );
        // 更新节点内容
        assert!(
            graph
                .update_entity_content(
                    id,
                    "New Content".to_string(),
                    default_distinct(),
                    &default_addons(),
                )
                .is_ok()
        );
        // 检查节点内容
        assert_eq!(
            graph.current.nodes.get(&id).unwrap().content,
            "New Content".to_string()
        );
        // 更新不存在的节点应该失败
        match graph.update_entity_content(
            999,
            "No Node".to_string(),
            default_distinct(),
            &default_addons(),
        ) {
            Err(GraphError::EntityNotFound(eid)) => assert_eq!(eid, 999),
            _ => panic!("Expected EntityNotFound error"),
        }
    }

    #[test]
    fn test_edge_operations() {
        let mut graph = KnowledgeGraph::default();
        let from = graph.add_entity(
            "From Node".to_string(),
            default_distinct(),
            &default_addons(),
            default_coor(),
        );
        let to = graph.add_entity(
            "To Node".to_string(),
            default_distinct(),
            &default_addons(),
            default_coor(),
        );

        // 添加边
        assert!(graph.add_edge(from, to, default_relation()).is_ok());
        assert!(graph.current.edges.contains_key(&(from, to)));

        // 添加重复的边
        assert!(graph.add_edge(from, to, Relation::Order).is_ok());
        assert!(graph.current.edges.contains_key(&(from, to)));

        // 更新边
        assert!(graph.update_edge(from, to, default_relation()).is_ok());

        // 删除边
        assert!(graph.remove_edge(from, to).is_ok());
        assert!(!graph.current.edges.contains_key(&(from, to)));

        // 删除不存在的边应该失败
        match graph.remove_edge(from, to) {
            Err(GraphError::EdgeNotFound(f, t)) => {
                assert_eq!(f, from);
                assert_eq!(t, to);
            }
            _ => panic!("Expected EdgeNotFound error"),
        }
    }

    #[test]
    fn test_undo_redo() {
        let mut graph = KnowledgeGraph::default();
        // 添加一个节点
        let id = graph.add_entity(
            "Undo Node".to_string(),
            default_distinct(),
            &default_addons(),
            default_coor(),
        );
        assert!(graph.current.nodes.contains_key(&id));
        // 撤回操作会移除节点
        assert!(graph.undo().is_ok());
        assert!(!graph.current.nodes.contains_key(&id));
        // 重做操作会恢复节点
        assert!(graph.redo().is_ok());
        assert!(graph.current.nodes.contains_key(&id));

        // 测试空的撤回栈
        let mut graph2 = KnowledgeGraph::default();
        match graph2.undo() {
            Err(GraphError::NothingToUndo) => {}
            _ => panic!("Expected NothingToUndo error"),
        }
        // 测试空的重做栈
        match graph2.redo() {
            Err(GraphError::NothingToRedo) => {}
            _ => panic!("Expected NothingToRedo error"),
        }
    }

    #[test]
    fn test_history_limit() {
        let mut graph = KnowledgeGraph::default();
        // 添加 5 个节点
        for i in 0..150 {
            graph.add_entity(
                format!("Node {}", i),
                default_distinct(),
                &default_addons(),
                default_coor(),
            );
        }
        // 撤回栈应该不超过 3
        assert!(graph.undo_stack.len() == 100);
        // 撤回尽可能多的次数，直到没有操作可撤回
        let mut undos = 0;
        while graph.undo().is_ok() {
            undos += 1;
        }
        // 撤回次数应该为 3
        assert!(undos == 100);
    }
}
