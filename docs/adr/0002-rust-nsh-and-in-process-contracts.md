# 采用纯 Rust NSH 与同进程共享契约包

Status: accepted

本应用需同时面向 Android、Windows、macOS 交付原生高性能界面，且端侧本地自治运行（无远端自建服务），没有动态下发脚本包的诉求。决定采用纯 Rust 模式的 Native Shell（NSH）构建客户端，并与同进程 Rust 后端直接共享零实现依赖的 `contracts` crate 作为南北向单一真源。

## 备选方案

- Rust 后端 + TSX 界面（NSH 混合模式）：可借助 QuickJS 支持热更新与动态包，但会引入 JS 运行时内存开销、跨语言类型生成桥接与双工具链维护成本，不符合本项目纯原生高性能定位。
- 跨进程 IPC 或本地 HTTP JSON 契约（如 OpenAPI / Protobuf）：在无独立进程隔离诉求的同语言单二进制内会增加序列化与生成样板，不如 Rust `contracts` crate 零漂移直接编译期校验。

## 影响

- 客户端与后端共享 `hg-contracts` crate，业务类型变更即时触发全仓编译期检查，零代码生成步骤。
- 仍须通过 `clients/gateway` 统一抽象与 `ast-grep` 规则隔离客户端与后端内部实现，确保可交互原型阶段与测试期可无感切换内存 Fake。
