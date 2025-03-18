/// 实体类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistinctEntityType {
    KnowledgeArena,  // 知识领域
    KnowledgeUnit,   // 知识单元
    KnowledgePoint,  // 知识点
    KnowledgeDetail, // 关键知识细节
    Video,           // 视频
    PowerPoint,      // PPT
    Document,        // 文档
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

/// 实体 ID
pub type EntityId = u64;
