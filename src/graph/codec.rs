//! 知识图谱编解码 XML 格式的定义与实现

use std::{collections::HashSet, io::Cursor};

use im::HashMap;
use quick_xml::{Reader, Writer, events::Event};
use serde::{Deserialize, Serialize};

use crate::error::SerdeError;

use super::{AddonEntityType, DistinctEntityType, EntityNode, Relation, Snapshot};

/// 转义非 ASCII 字符
fn escape_non_ascii(input: &str) -> String {
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
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "entity")]
struct SerializableEntity {
    id: u64,
    class_name: String,
    classification: String,
    identity: String,
    level: String,
    #[serde(
        serialize_with = "serialize_addon_types",
        deserialize_with = "deserialize_addon_types"
    )]
    attach: HashSet<AddonEntityType>,
    opentool: String,
    content: String,
    x: f64,
    y: f64,
}

impl Default for SerializableEntity {
    fn default() -> Self {
        Self {
            id: 0,
            class_name: String::new(),
            classification: "内容方法型节点".to_string(),
            identity: "知识".to_string(),
            level: String::new(),
            attach: HashSet::new(),
            opentool: "无".to_string(),
            content: String::new(),
            x: 0.0,
            y: 0.0,
        }
    }
}

impl From<&EntityNode> for SerializableEntity {
    fn from(node: &EntityNode) -> Self {
        let distinct_type = node.distinct_type;
        let coor = node.coor;

        Self {
            id: node.id,
            class_name: distinct_type.class_name().to_string(),
            level: distinct_type.level().to_string(),
            attach: node.addon_types.clone(),
            content: node.content.to_string(),
            x: coor.0,
            y: coor.1,
            ..Default::default()
        }
    }
}

impl TryFrom<SerializableEntity> for EntityNode {
    type Error = SerdeError;
    fn try_from(value: SerializableEntity) -> Result<Self, Self::Error> {
        // 根据 class_name 确定实体类型
        let distinct_type = match value.class_name.as_str() {
            "知识领域" => DistinctEntityType::KnowledgeArena,
            "知识单元" => DistinctEntityType::KnowledgeUnit,
            "知识点" => DistinctEntityType::KnowledgePoint,
            "关键知识细节" => DistinctEntityType::KnowledgeDetail,
            value_name => {
                return Err(SerdeError::Unexpected("实体类型", value_name.to_string()));
            }
        };

        Ok(Self::new(
            value.id,
            value.content,
            distinct_type,
            &value.attach.iter().copied().collect::<Vec<_>>(),
            (value.x, value.y),
        ))
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

/// 附加实体类型，顺序是固定的，即 T Z Q K E P
const ADDON_TYPES: [AddonEntityType; 6] = [
    AddonEntityType::Thinking,
    AddonEntityType::Political,
    AddonEntityType::Question,
    AddonEntityType::Knowledge,
    AddonEntityType::Example,
    AddonEntityType::Practice,
];

/// 序列化附加实体类型
fn serialize_addon_types<S>(
    addon_types: &HashSet<AddonEntityType>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut result = String::with_capacity(6);

    // 根据 addon 是否在 addon_types 中决定是否添加对应的字符
    for addon in ADDON_TYPES.iter() {
        result.push(if addon_types.contains(addon) {
            '1'
        } else {
            '0'
        });
    }

    serializer.serialize_str(&result)
}

fn deserialize_addon_types<'de, D>(deserializer: D) -> Result<HashSet<AddonEntityType>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // 将输入字符串反序列化
    let s = String::deserialize(deserializer)?;
    let mut set = HashSet::new();

    // 根据字符是否为'1'决定是否添加对应的 addon
    for (c, addon) in s.chars().zip(ADDON_TYPES.iter()) {
        if c == '1' {
            set.insert(*addon);
        }
    }
    Ok(set)
}

/// 可序列化的边
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "relation")]
struct SerializableEdge {
    name: String,
    headnodeid: u64,
    tailnodeid: u64,
    class_name: String,
    mask: String,
    classification: String,
    head_need: String,
    tail_need: String,
}

impl Default for SerializableEdge {
    fn default() -> Self {
        Self {
            name: "包含".to_string(),
            headnodeid: 0,
            tailnodeid: 0,
            class_name: String::new(),
            mask: "知识连线".to_string(),
            classification: String::new(),
            head_need: "内容方法型节点".to_string(),
            tail_need: "内容方法型节点".to_string(),
        }
    }
}

impl SerializableEdge {
    /// 从边创建可序列化的边
    pub fn from_edge(from: u64, to: u64, relation: Relation) -> Self {
        Self {
            headnodeid: from,
            tailnodeid: to,
            class_name: relation.class_name().to_string(),
            classification: relation.classification().to_string(),
            ..Default::default()
        }
    }

    /// 将可序列化的边转换为边
    pub fn to_edge(&self) -> Result<(u64, u64, Relation), SerdeError> {
        let relation = match self.class_name.as_str() {
            "包含关系" => Relation::Contain,
            "次序关系" | "次序：次序关系" => Relation::Order,
            _ => {
                return Err(SerdeError::Unexpected("关系名", self.class_name.clone()));
            }
        };

        Ok((self.headnodeid, self.tailnodeid, relation))
    }
}

impl Relation {
    /// 获取关系 class_name
    fn class_name(&self) -> &'static str {
        match *self {
            Relation::Contain => "包含关系",
            Relation::Order => "次序关系",
        }
    }

    /// 获取关系 classification
    fn classification(&self) -> &'static str {
        match *self {
            Relation::Contain => "包含关系",
            Relation::Order => "次序关系",
        }
    }
}

/// 可序列化的快照
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "KG")]
pub struct SerializableSnapshot {
    #[serde(rename = "$value")]
    title: String,
    entities: Entities,
    relations: Relations,
}

/// 实体包装器
#[derive(Debug, Serialize, Deserialize)]
struct Entities {
    #[serde(rename = "entity", default)]
    entities: Vec<SerializableEntity>,
}

/// 关系包装器
#[derive(Debug, Serialize, Deserialize)]
struct Relations {
    #[serde(rename = "relation", default)]
    pub items: Vec<SerializableEdge>,
}

impl From<&Snapshot> for SerializableSnapshot {
    fn from(value: &Snapshot) -> Self {
        // 将实体节点转换为可序列化的实体节点
        let entities = value
            .nodes
            .iter()
            .map(|(_, node)| SerializableEntity::from(node))
            .collect();

        // 将边转换为可序列化的边
        let relations = value
            .edges
            .iter()
            .map(|(&(head, tail), relation)| SerializableEdge::from_edge(head, tail, *relation))
            .collect();

        Self {
            title: "教学知识图谱".to_string(),
            entities: Entities { entities },
            relations: Relations { items: relations },
        }
    }
}

impl TryFrom<SerializableSnapshot> for Snapshot {
    type Error = SerdeError;

    fn try_from(value: SerializableSnapshot) -> Result<Self, Self::Error> {
        // 将实体节点转换为哈希表
        let nodes: HashMap<_, _> = value
            .entities
            .entities
            .into_iter()
            .map(|entity| {
                let entity = EntityNode::try_from(entity)?;
                Ok::<_, SerdeError>((entity.id, entity))
            })
            .collect::<Result<_, _>>()?;

        // 将边转换为哈希表
        let edges = value
            .relations
            .items
            .into_iter()
            .map(|edge| {
                let (from, to, relation) = edge.to_edge()?;
                Ok::<_, SerdeError>(((from, to), relation))
            })
            .collect::<Result<_, _>>()?;

        // 获取最大的节点 ID
        let latest_id = nodes.keys().max().copied().unwrap_or(0) + 1;

        Ok(Self {
            nodes,
            edges,
            latest_id,
        })
    }
}

fn indent_xml(xml_string: &str) -> Result<String, quick_xml::Error> {
    let mut reader = Reader::from_str(xml_string);

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 4);

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(event) => {
                writer.write_event(event)?;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(String::from_utf8(writer.into_inner().into_inner()).unwrap())
}

impl SerializableSnapshot {
    /// 将快照转换为 XML 格式
    pub fn to_xml(&self) -> Result<String, SerdeError> {
        // 序列化为 XML 字符串
        let content = quick_xml::se::to_string(self)?;

        // 添加缩进
        let indented_content = indent_xml(&content)?;

        // 转义非 ASCII 字符
        Ok(escape_non_ascii(&indented_content))
    }

    /// 从 XML 字符串解析快照
    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::DeError> {
        // 解析 XML 字符串
        quick_xml::de::from_str(xml)
    }
}

impl Snapshot {
    /// 将快照转换为 XML 格式
    #[inline]
    pub fn to_xml(&self) -> Result<String, SerdeError> {
        SerializableSnapshot::from(self).to_xml()
    }

    /// 从 XML 字符串解析快照
    #[inline]
    pub fn from_xml(xml: &str) -> Result<Self, SerdeError> {
        let s = SerializableSnapshot::from_xml(xml).map_err(SerdeError::Deserialize)?;
        Snapshot::try_from(s)
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
        ];
        let xmls = [
            "<relation><name>&#21253;&#21547;</name><headnodeid>114514</headnodeid><tailnodeid>1919810</tailnodeid><class_name>&#21253;&#21547;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#21253;&#21547;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>",
            "<relation><name>&#21253;&#21547;</name><headnodeid>114514</headnodeid><tailnodeid>1919810</tailnodeid><class_name>&#27425;&#24207;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#27425;&#24207;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>",
        ];
        for (((head, tail), relation), xml_gt) in relations.iter().zip(xmls.iter()) {
            let xml = to_xml(SerializableEdge::from_edge(*head, *tail, *relation)).unwrap();
            assert_eq!(xml, *xml_gt);
        }
    }

    fn create_knowledge_graph() -> Result<KnowledgeGraph, Box<dyn std::error::Error>> {
        let mut knowledge_graph = KnowledgeGraph::default();

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

        Ok(knowledge_graph)
    }

    #[test]
    fn test_encode_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let knowledge_graph = create_knowledge_graph()?;
        let xml = knowledge_graph
            .current_snapshot()
            .to_xml()?
            .replace(['\n', ' '], "");

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
        assert!(xml.contains("<relation><name>&#21253;&#21547;</name><headnodeid>3</headnodeid><tailnodeid>4</tailnodeid><class_name>&#27425;&#24207;&#20851;&#31995;</class_name><mask>&#30693;&#35782;&#36830;&#32447;</mask><classification>&#27425;&#24207;&#20851;&#31995;</classification><head_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</head_need><tail_need>&#20869;&#23481;&#26041;&#27861;&#22411;&#33410;&#28857;</tail_need></relation>"));

        Ok(())
    }

    #[test]
    fn test_decode_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let knowledge_graph = create_knowledge_graph()?;
        let snapshot = knowledge_graph.current_snapshot();
        let xml = snapshot.to_xml()?;

        let snapshot_decoded = Snapshot::from_xml(&xml)?;
        assert_eq!(*snapshot, snapshot_decoded);

        Ok(())
    }
}
