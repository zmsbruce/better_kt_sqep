//! 图节点模块，包括实体节点、关系节点。

use std::collections::HashSet;

/// 实体节点
#[derive(Debug, Clone, PartialEq)]
pub struct EntityNode {
    pub id: u64,
    pub content: String,
    pub distinct_type: DistinctEntityType,
    pub addon_types: HashSet<AddonEntityType>,
    pub coor: (f64, f64),
}

impl EntityNode {
    /// 创建一个新的实体节点
    pub fn new(
        id: u64,
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
        coor: (f64, f64),
    ) -> Self {
        Self {
            id,
            content,
            distinct_type,
            addon_types: addon_types.iter().copied().collect(),
            coor,
        }
    }

    /// 修改实体节点内容。
    /// 注意：由于 ID 为图谱查找的键，因此 ID 不可修改。
    pub fn update(
        &mut self,
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
        coor: (f64, f64),
    ) {
        self.content = content;
        self.distinct_type = distinct_type;
        self.addon_types = addon_types.iter().copied().collect();
        self.coor = coor;
    }
}

// 关系类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
    Contain, // 包含关系
    Order,   // 次序关系
}

/// 实体类型
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum DistinctEntityType {
    KnowledgeArena,  // 知识领域
    KnowledgeUnit,   // 知识单元
    KnowledgePoint,  // 知识点
    KnowledgeDetail, // 关键知识细节
}

/// 附加实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddonEntityType {
    Knowledge, // 知识
    Thinking,  // 思维
    Example,   // 示例
    Question,  // 问题
    Practice,  // 练习
    Political, // 思政
}
