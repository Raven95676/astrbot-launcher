# AstrBot Launcher

AstrBot Launcher是一款用于图形化管理AstrBot的桌面应用程序，提供版本下载、实例管理、数据备份以及Python运行环境自动化配置等完整功能支持。

## 功能特性

- 零侵入架构设计：运行环境与依赖统一在独立目录管理，避免污染系统
- 多实例可视化管理：创建/启动/停止/升级一站式完成
- 安全备份恢复：实例级备份与恢复，数据更安心
- 运行时隔离：实例独立运行，杜绝环境冲突
- 桌面友好集成：托盘驻留、开机自启即装即用

## 技术栈

- 前端: React 19, Vite, Ant Design, TypeScript
- 后端: Rust + Tauri 2

## 安全性说明

本项目所有源代码公开，内嵌二进制文件ctrlc_sender.exe源码托管于<https://codeberg.org/Raven95676/ctrlc_sender>
