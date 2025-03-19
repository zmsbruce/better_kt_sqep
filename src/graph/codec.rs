//! 知识图谱编解码 XML 格式的定义与实现

use std::collections::HashSet;

use lazy_static::lazy_static;
use quick_xml::se::to_string as serialize_xml;
use serde::Serialize;

use super::{AddonEntityType, DistinctEntityType, EntityNode};

lazy_static! {
    // 空附加实体类型，用于默认值
    static ref EMPTY_ADDONS: HashSet<AddonEntityType> = HashSet::new();
}

/// 可序列化的实体节点
#[derive(Debug, Serialize)]
#[serde(rename = "entity")]
pub struct SerializableEntity<'a> {
    id: u64,
    class_name: &'static str,
    classification: &'static str,
    identity: &'static str,
    level: &'static str,
    #[serde(serialize_with = "serialize_addon_types")]
    attach: &'a HashSet<AddonEntityType>,
    opentool: &'static str,
    content: &'a str,
    x: f64,
    y: f64,
}

impl Default for SerializableEntity<'_> {
    fn default() -> Self {
        Self {
            id: 0,
            class_name: "",
            classification: "内容方法型节点",
            identity: "知识",
            level: "",
            attach: &EMPTY_ADDONS,
            opentool: "无",
            content: "",
            x: 0.0,
            y: 0.0,
        }
    }
}

impl SerializableEntity<'_> {
    /// 序列化为 XML 格式
    pub fn to_xml(&self) -> Result<String, quick_xml::SeError> {
        // 序列化 content 字段
        let content = serialize_xml(self)?;

        // 将 content 中的非 ASCII 字符转义
        Ok(content
            .chars()
            .map(|c| {
                if c.is_ascii() {
                    c.to_string()
                } else {
                    format!("&#{};", c as u32)
                }
            })
            .collect())
    }
}

impl<'a> From<&'a EntityNode> for SerializableEntity<'a> {
    fn from(node: &'a EntityNode) -> Self {
        let distinct_type = node.distinct_type();
        let coor = node.coor();

        Self {
            id: node.id(),
            class_name: distinct_type.class_name(),
            level: distinct_type.level(),
            attach: node.addon_types(),
            content: node.content(),
            x: coor.0,
            y: coor.1,
            ..Default::default()
        }
    }
}

/// 实体的 class_name, classification, identity, level, opentool 和实体类型是一一对应的
impl DistinctEntityType {
    /// 获取实体类型 class_name
    fn class_name(&self) -> &'static str {
        match *self {
            DistinctEntityType::KnowledgeArena => "知识领域",
            DistinctEntityType::KnowledgeUnit => "知识单元",
            DistinctEntityType::KnowledgePoint => "知识点",
            DistinctEntityType::KnowledgeDetail => "关键知识细节",
        }
    }

    /// 获取实体类型 level
    fn level(&self) -> &'static str {
        match *self {
            DistinctEntityType::KnowledgeArena => "一级",
            DistinctEntityType::KnowledgeUnit => "二级",
            DistinctEntityType::KnowledgePoint => "归纳级",
            DistinctEntityType::KnowledgeDetail => "内容级",
        }
    }
}

/// 序列化附加实体类型
fn serialize_addon_types<S>(
    addon_types: &HashSet<AddonEntityType>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut result = String::with_capacity(6);

    for addon in [
        AddonEntityType::Thinking,
        AddonEntityType::Political,
        AddonEntityType::Question,
        AddonEntityType::Knowledge,
        AddonEntityType::Example,
        AddonEntityType::Practice,
    ]
    .iter()
    // 顺序是固定的，即 T Z Q K E P
    {
        result.push(if addon_types.contains(addon) {
            '1'
        } else {
            '0'
        });
    }

    serializer.serialize_str(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_addons() -> Vec<AddonEntityType> {
        vec![
            AddonEntityType::Thinking,
            AddonEntityType::Political,
            AddonEntityType::Question,
        ]
    }

    fn default_coordinate() -> (f64, f64) {
        (1.0, 2.0)
    }

    fn default_content() -> String {
        "Hello 世界！🦀@#& ".to_string()
    }

    fn default_id() -> u64 {
        114514
    }

    #[test]
    fn test_encode_entity_node() {
        let distinct_types = [
            DistinctEntityType::KnowledgeArena,
            DistinctEntityType::KnowledgeUnit,
            DistinctEntityType::KnowledgePoint,
            DistinctEntityType::KnowledgeDetail,
        ];

        let xmls = [
            "<entity><id>114514</id><class_name>&#30693;&#35782;&#39046;&#22495;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#19968;&#32423;</level><attach>111000</attach><opentool>&#26080;</opentool><content>Hello &#19990;&#30028;&#65281;&#129408;@#&amp; </content><x>1</x><y>2</y></entity>",
            "<entity><id>114514</id><class_name>&#30693;&#35782;&#21333;&#20803;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#20108;&#32423;</level><attach>111000</attach><opentool>&#26080;</opentool><content>Hello &#19990;&#30028;&#65281;&#129408;@#&amp; </content><x>1</x><y>2</y></entity>",
            "<entity><id>114514</id><class_name>&#30693;&#35782;&#28857;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#24402;&#32435;&#32423;</level><attach>111000</attach><opentool>&#26080;</opentool><content>Hello &#19990;&#30028;&#65281;&#129408;@#&amp; </content><x>1</x><y>2</y></entity>",
            "<entity><id>114514</id><class_name>&#20851;&#38190;&#30693;&#35782;&#32454;&#33410;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#20869;&#23481;&#32423;</level><attach>111000</attach><opentool>&#26080;</opentool><content>Hello &#19990;&#30028;&#65281;&#129408;@#&amp; </content><x>1</x><y>2</y></entity>",
        ];

        for (distinct_type, xml_gt) in distinct_types.iter().zip(xmls.iter()) {
            let node = EntityNode::new(
                default_id(),
                default_content(),
                *distinct_type,
                &default_addons(),
                default_coordinate(),
            );

            let xml = SerializableEntity::from(&node).to_xml().unwrap();
            assert_eq!(xml, *xml_gt);
        }
    }
}
