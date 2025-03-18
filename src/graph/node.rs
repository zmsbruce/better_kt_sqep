use std::collections::HashSet;

use super::types::{AddonEntityType, DistinctEntityType};

/// 实体节点
#[derive(Debug, Clone)]
pub struct EntityNode {
    content: String,
    distinct_type: DistinctEntityType,
    addon_types: HashSet<AddonEntityType>,
}

impl EntityNode {
    /// 创建一个新的实体节点
    pub fn new(
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
    ) -> Self {
        Self {
            content,
            distinct_type,
            addon_types: addon_types.iter().copied().collect(),
        }
    }

    /// 修改实体节点内容
    pub fn update(
        &mut self,
        content: String,
        distinct_type: DistinctEntityType,
        addon_types: &[AddonEntityType],
    ) {
        self.content = content;
        self.distinct_type = distinct_type;
        self.addon_types = addon_types.iter().copied().collect();
    }

    /// 获取实体节点内容
    #[inline]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// 获取实体节点类型
    #[inline]
    pub fn distinct_type(&self) -> DistinctEntityType {
        self.distinct_type
    }

    /// 获取实体节点附加类型
    #[inline]
    pub fn addon_types(&self) -> &HashSet<AddonEntityType> {
        &self.addon_types
    }
}

// 关系类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
    Contain,         // 包含关系
    Order,           // 次序关系
    KeyOrder,        // 关键次序
    ResourceConnect, // 连接资源
}
