# Better KT-SQEP

一个 **兼容 KT-SQEP**，但更**稳定、更人性化、更快的**知识图谱绘制工具。

> 注：KT-SQEP 是一个知识图谱工具，用于战德臣老师所授课程的知识图谱绘制要求。

![展示](assets/image.png)

### 如果对您有所帮助，希望点一个免费的 star⭐，谢谢！

## 有关课程

哈尔滨工业大学

- CS64006	高级数据库系统
- CS65003	企业资源规划与供应链管理系统
- *...... 欢迎进行补充*

## 特点

- **不闪退，不闪退，不闪退**
- 支持自动保存
- 支持撤销与恢复
- 支持快捷键
- 操作方式更人性化
- 性能更高
- 单个程序即可运行

## 下载

[点击前往下载](https://github.com/zmsbruce/better_kt_sqep/releases)

## 操作方式

见 [Usage.mp4](./Usage.mp4)

## 使用 Python 绑定

克隆本项目后，在控制台输入：

```sh
# 创建一个虚拟环境
python -m venv .venv

# 激活虚拟环境（Linux）
source .venv/bin/activate

# 或者激活虚拟环境（Windows）
.venv\Scripts\activate

pip install maturin
maturin develop
```

之后您可以使用 Python 绑定，使用方法如下：

```python
from py_better_kt_sqep import KnowledgeGraph

# 初始化一个知识图谱
kg = KnowledgeGraph()

# 添加节点
#
# 参数：
# - content：节点内容
# - distinct_type：独立实体类型，分别是 ka (知识领域)、ku (知识单元)、kp (知识点)、kd (知识细节)
# - addon_types：附加实体类型，为 k (知识)、t (思维)、e (示例)、q (问题)、p (练习)、z (思政) 的组合
# - x：横坐标
# - y：纵坐标
#
# 返回：节点 id
entity_1 = kg.add_entity("这里是节点一", "ka", "kte", 0.0, 100.0)
entity_2 = kg.add_entity("这里是节点二", "ka", "kte", 100.0, 100.0)

# 添加边
#
# 参数：
# - from：边开始的节点 id
# - to：边指向的节点 id
# - relation：关系，为 contain (包含) 或者 order 次序
kg.add_edge(entity_1, entity_2, "contain")

# 删除边
#
# 参数：
# - from：边开始的节点 id
# - to：边指向的节点 id
kg.remove_edge(entity_1, entity_2)

# 删除节点
#
# 参数：
# - id：节点 id
kg.remove_entity(entity_2)

# 导出为 XML
#
# 返回：XML 字符串
xml = kg.to_xml()
```

## 编译

如果下载网页中没有您需要的版本，您可以克隆本项目然后编译安装。

首先需要安装 [Rust](https://www.rust-lang.org/zh-CN/tools/install) 。安装完毕之后，在项目所在目录输入：

```bash
cargo build --release
```

即编译完成。目标文件为 /target/release/better_kt_sqep

## 兼容情况

支持 KT-SQEP 的教学知识图谱，包括：

- 内容型独立实体
  - 知识领域
  - 知识单元
  - 知识点
  - 关键知识细节
- 附加实体类型
  - 知识 K
  - 思维 T
  - 示例 E
  - 问题 Q
  - 练习 P
  - 思政 Z
- 关系类型
  - 包含关系
  - 次序：次序关系

暂不支持：

- 能力知识图谱；
- 资源型独立实体；
- 关系类型
  - 次序：关键次序
  - 连接资源