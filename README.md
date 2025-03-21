# Better KT-SQEP

一个 **兼容 KT-SQEP**，但更稳定、更人性化、更快的知识图谱绘制工具。

![展示](assets/image.png)

### 如果对您有所帮助，希望点一个免费的 star⭐，谢谢喵！

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

## 编译安装

如果下载网页中没有您需要的版本，您可以克隆本项目然后编译安装。

首先需要安装 [Rust](https://www.rust-lang.org/zh-CN/tools/install)。安装完毕之后，在项目所在目录输入：

```bash
cargo build --release
```

目标文件为 /target/release/better_kt_sqep

## 操作方式

见 [Usage.mp4](./Usage.mp4)

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