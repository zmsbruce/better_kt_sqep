//! 知识图谱编解码 XML 格式的定义与实现

use std::collections::HashSet;

use lazy_static::lazy_static;
use serde::Serialize;

use super::{AddonEntityType, DistinctEntityType, EntityNode, Relation, Snapshot};

lazy_static! {
    // 空附加实体类型，用于默认值
    static ref EMPTY_ADDONS: HashSet<AddonEntityType> = HashSet::new();
}

/// 转义非 ASCII 字符
pub fn escape_non_ascii(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii() {
                c.to_string()
            } else {
                format!("&#{};", c as u32)
            }
        })
        .collect()
}

/// 可序列化的实体节点
#[derive(Debug, Serialize)]
#[serde(rename = "entity")]
struct SerializableEntity<'a> {
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

/// 可序列化的关系
#[derive(Debug, Serialize)]
#[serde(rename = "relation")]
struct SerializableRelation {
    name: &'static str,
    headnodeid: u64,
    tailnodeid: u64,
    class_name: &'static str,
    mask: &'static str,
    classification: &'static str,
    head_need: &'static str,
    tail_need: &'static str,
}

impl Default for SerializableRelation {
    fn default() -> Self {
        Self {
            name: "包含",
            headnodeid: 0,
            tailnodeid: 0,
            class_name: "",
            mask: "知识连线",
            classification: "",
            head_need: "内容方法型节点",
            tail_need: "内容方法型节点",
        }
    }
}

impl From<&((u64, u64), Relation)> for SerializableRelation {
    fn from(value: &((u64, u64), Relation)) -> Self {
        let ((head_id, tail_id), relation) = value;

        Self {
            headnodeid: *head_id,
            tailnodeid: *tail_id,
            class_name: relation.class_name(),
            classification: relation.classification(),
            ..Default::default()
        }
    }
}

impl Relation {
    /// 获取关系 class_name
    fn class_name(&self) -> &'static str {
        match *self {
            Relation::Contain => "包含关系",
            Relation::Order => "次序：次序关系",
            Relation::KeyOrder => "次序：关键次序",
        }
    }

    /// 获取关系 classification
    fn classification(&self) -> &'static str {
        match *self {
            Relation::Contain => "包含关系",
            Relation::Order | Relation::KeyOrder => "次序关系",
        }
    }
}

/// 可序列化的快照
#[derive(Debug, Serialize)]
#[serde(rename = "KG")]
struct SerializableSnapshot<'a> {
    #[serde(rename = "$value")]
    title: &'static str,
    entities: Entities<'a>,
    relations: Relations,
}

/// 实体包装器
#[derive(Debug, Serialize)]
struct Entities<'a> {
    #[serde(rename = "entity")]
    entities: Vec<SerializableEntity<'a>>,
}

/// 关系包装器
#[derive(Debug, Serialize)]
struct Relations {
    #[serde(rename = "relation")]
    pub items: Vec<SerializableRelation>,
}

impl<'a> From<&'a Snapshot> for SerializableSnapshot<'a> {
    fn from(value: &'a Snapshot) -> Self {
        let entities = value
            .nodes
            .iter()
            .map(|(_, node)| SerializableEntity::from(node))
            .collect();
        let relations = value
            .edges
            .iter()
            .map(|(&(head, tail), relation)| SerializableRelation::from(&((head, tail), *relation)))
            .collect();

        Self {
            title: "教学知识图谱",
            entities: Entities { entities },
            relations: Relations { items: relations },
        }
    }
}

impl SerializableSnapshot<'_> {
    /// 将快照转换为 XML 格式
    pub fn to_xml(&self) -> Result<String, quick_xml::SeError> {
        // 序列化为 XML 字符串
        let content = quick_xml::se::to_string(self)?;

        // 转义非 ASCII 字符
        Ok(escape_non_ascii(&content))
    }
}

impl Snapshot {
    /// 将快照转换为 XML 格式
    pub fn to_xml(&self) -> Result<String, quick_xml::SeError> {
        SerializableSnapshot::from(self).to_xml()
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::KnowledgeGraph;

    use super::*;

    use quick_xml::se::to_string as serialize_xml;

    fn to_xml(value: impl Serialize) -> Result<String, quick_xml::SeError> {
        let content = serialize_xml(&value)?;
        Ok(escape_non_ascii(&content))
    }

    #[test]
    fn test_encode_entity_node() {
        let default_id = 114514;
        let default_content = "Hello 世界！🦀@#& ";
        let default_addons = vec![
            AddonEntityType::Thinking,
            AddonEntityType::Political,
            AddonEntityType::Question,
        ];
        let default_coordinate = (1.0, 2.0);

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
                default_id,
                default_content.to_string(),
                *distinct_type,
                &default_addons,
                default_coordinate,
            );

            let xml = to_xml(SerializableEntity::from(&node)).unwrap();
            assert_eq!(xml, *xml_gt);
        }
    }

    #[test]
    fn test_encode_relation() {
        let (id_1, id_2) = (114514, 1919810);
        let relations = [
            ((id_1, id_2), Relation::Contain),
            ((id_1, id_2), Relation::Order),
            ((id_1, id_2), Relation::KeyOrder),
        ];
        let xmls = [
            "<relation><name>&#21253;&#21547;</name><headnodeid>114514</headnodeid><tailnodeid>1919810</tailnodeid><class_name>&#21253;&#21547;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#21253;&#21547;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>",
            "<relation><name>&#21253;&#21547;</name><headnodeid>114514</headnodeid><tailnodeid>1919810</tailnodeid><class_name>&#27425;&#24207;&#65306;&#27425;&#24207;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#27425;&#24207;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>",
            "<relation><name>&#21253;&#21547;</name><headnodeid>114514</headnodeid><tailnodeid>1919810</tailnodeid><class_name>&#27425;&#24207;&#65306;&#20851;&#38190;&#27425;&#24207;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#27425;&#24207;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>",
        ];
        for (relation, xml_gt) in relations.iter().zip(xmls.iter()) {
            let xml = to_xml(SerializableRelation::from(relation)).unwrap();
            assert_eq!(xml, *xml_gt);
        }
    }

    #[test]
    fn test_encode_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let mut knowledge_graph = KnowledgeGraph::new(0);

        let id_1 = knowledge_graph.add_entity(
            "什么是计算思维".to_string(),
            DistinctEntityType::KnowledgeArena,
            &[AddonEntityType::Thinking],
            (0.0, 0.0),
        );
        let id_2 = knowledge_graph.add_entity(
            "典型的计算思维".to_string(),
            DistinctEntityType::KnowledgePoint,
            &[
                AddonEntityType::Thinking,
                AddonEntityType::Example,
                AddonEntityType::Question,
            ],
            (1.0, 1.0),
        );
        let id_3 = knowledge_graph.add_entity(
            "小白鼠检验毒水瓶问题,怎样求解？".to_string(),
            DistinctEntityType::KnowledgeDetail,
            &[
                AddonEntityType::Practice,
                AddonEntityType::Example,
                AddonEntityType::Question,
            ],
            (2.0, 2.0),
        );
        let id_4 = knowledge_graph.add_entity(
            "水瓶编号：由十进制编号到二进制编号".to_string(),
            DistinctEntityType::KnowledgeDetail,
            &[
                AddonEntityType::Practice,
                AddonEntityType::Example,
                AddonEntityType::Thinking,
            ],
            (3.0, 3.0),
        );
        knowledge_graph.add_edge(id_1, id_2, Relation::Contain)?;
        knowledge_graph.add_edge(id_1, id_3, Relation::Contain)?;
        knowledge_graph.add_edge(id_3, id_4, Relation::Order)?;

        let xml = knowledge_graph.current_snapshot().to_xml()?;

        // 检查 XML 结构是否正确
        let pattern = r"^<KG>&#25945;&#23398;&#30693;&#35782;&#22270;&#35889;<entities>(?:<entity>.*?</entity>)+</entities><relations>(?:<relation>.*?</relation>)+</relations></KG>$";
        assert!(regex::Regex::new(pattern)?.is_match(&xml));

        // 检查 XML 内容是否正确
        assert!(xml.contains("<entity><id>1</id><class_name>&#30693;&#35782;&#39046;&#22495;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#19968;&#32423;</level><attach>100000</attach><opentool>&#26080;</opentool><content>&#20160;&#20040;&#26159;&#35745;&#31639;&#24605;&#32500;</content><x>0</x><y>0</y></entity>"));
        assert!(xml.contains("<entity><id>2</id><class_name>&#30693;&#35782;&#28857;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#24402;&#32435;&#32423;</level><attach>101010</attach><opentool>&#26080;</opentool><content>&#20856;&#22411;&#30340;&#35745;&#31639;&#24605;&#32500;</content><x>1</x><y>1</y></entity>"));
        assert!(xml.contains("<entity><id>3</id><class_name>&#20851;&#38190;&#30693;&#35782;&#32454;&#33410;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#20869;&#23481;&#32423;</level><attach>001011</attach><opentool>&#26080;</opentool><content>&#23567;&#30333;&#40736;&#26816;&#39564;&#27602;&#27700;&#29942;&#38382;&#39064;,&#24590;&#26679;&#27714;&#35299;&#65311;</content><x>2</x><y>2</y></entity>"));
        assert!(xml.contains("<entity><id>4</id><class_name>&#20851;&#38190;&#30693;&#35782;&#32454;&#33410;</class_name><classification>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</classification><identity>&#30693;&#35782;</identity><level>&#20869;&#23481;&#32423;</level><attach>100011</attach><opentool>&#26080;</opentool><content>&#27700;&#29942;&#32534;&#21495;&#65306;&#30001;&#21313;&#36827;&#21046;&#32534;&#21495;&#21040;&#20108;&#36827;&#21046;&#32534;&#21495;</content><x>3</x><y>3</y></entity>"));
        assert!(xml.contains("<relation><name>&#21253;&#21547;</name><headnodeid>1</headnodeid><tailnodeid>2</tailnodeid><class_name>&#21253;&#21547;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#21253;&#21547;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>"));
        assert!(xml.contains("<relation><name>&#21253;&#21547;</name><headnodeid>1</headnodeid><tailnodeid>3</tailnodeid><class_name>&#21253;&#21547;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#21253;&#21547;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>"));
        assert!(xml.contains("<relation><name>&#21253;&#21547;</name><headnodeid>3</headnodeid><tailnodeid>4</tailnodeid><class_name>&#27425;&#24207;&#65306;&#27425;&#24207;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#27425;&#24207;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>"));

        Ok(())
    }
}
