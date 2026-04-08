# macOS 代码签名配置指南

## 为什么需要签名？

未签名的 macOS 应用会被 Gatekeeper 阻止，提示"已损坏，无法打开"。需要代码签名 + 公证（notarization）才能让用户正常安装。

## 前置准备

### 1. 加入 Apple Developer Program
- 访问 https://developer.apple.com/programs/
- 费用：$99/年（个人或组织）

### 2. 创建证书（Developer ID Application）
1. 打开 Xcode → Settings → Accounts → 添加 Apple ID
2. Manage Certificates → + → Developer ID Application
3. 导出证书：
   ```bash
   # 在 Keychain Access 中找到证书
   # 右键 → 导出 → 保存为 Certificates.p12
   # 设置密码（记住，后面要用）
   ```

### 3. 创建 App-Specific Password（用于公证）
1. 访问 https://appleid.apple.com/account/manage
2. 安全 → App-Specific Passwords → Generate Password
3. 保存密码（后面要用）

## GitHub Secrets 配置

在仓库 Settings → Secrets and variables → Actions → New repository secret 中添加：

| Secret Name | 说明 | 如何获取 |
|---|---|---|
| `APPLE_CERTIFICATE` | Base64 编码的 .p12 证书 | `base64 -i Certificates.p12 | pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | .p12 证书导出时的密码 | 导出证书时设置的密码 |
| `APPLE_SIGNING_IDENTITY` | 证书名称 | `Developer ID Application: Your Name (TEAM_ID)` |
| `APPLE_ID` | Apple ID 邮箱 | 你的 Apple 账号邮箱 |
| `APPLE_PASSWORD` | App-Specific Password | 第 3 步生成的密码 |
| `APPLE_TEAM_ID` | Team ID | https://developer.apple.com/account/ → Membership → Team ID |

### 获取 APPLE_SIGNING_IDENTITY

```bash
# 查看所有可用的签名身份
security find-identity -v -p codesigning

# 输出示例：
# 1) 1234567890ABCDEF "Developer ID Application: Your Name (ABCD123456)"
# 复制整个名称（包括引号里的内容，不含引号本身）
```

## 本地测试签名

```bash
# 确保��书已安装
security find-identity -v -p codesigning

# 手动签名（测试）
codesign --force --deep --sign "Developer ID Application: Your Name (TEAM_ID)" \
  /path/to/ChangYan.app

# 验证签名
codesign --verify --deep --strict --verbose=2 /path/to/ChangYan.app
spctl -a -t exec -vv /path/to/ChangYan.app
```

## 验证 CI 签名是否生效

1. 推送 tag 触发 release workflow
2. 下载生成的 `.dmg` 或 `.app`
3. 运行验证命令：
   ```bash
   codesign -dvv /path/to/ChangYan.app
   # 应该看到 "Authority=Developer ID Application: Your Name"

   spctl -a -t exec -vv /path/to/ChangYan.app
   # 应该看到 "accepted" 或 "source=Notarized Developer ID"
   ```

## 故障排除

### 签名失败
- 检查 `APPLE_CERTIFICATE` 是否正确 base64 编码
- 检查 `APPLE_SIGNING_IDENTITY` 名称是否完全匹配（包括 Team ID）
- 确认证书未过期：`security find-certificate -c "Developer ID Application"`

### 公证失败
- 检查 `APPLE_ID` 和 `APPLE_PASSWORD` 是否正确
- 确认使用的是 App-Specific Password（不是 Apple ID 密码）
- 检查 `APPLE_TEAM_ID` 是否正确

### 仍然提示"已损坏"
- 如果签名成功但公证失败，用户可以手动移除隔离属性：
  ```bash
  xattr -cr /Applications/ChangYan.app
  ```
- 确保 `tauri.conf.json` 中的 `identifier` 格式正确（`com.changyan.app`）

## 无签名时的临时解决方案

如果暂时无法配置签名，在 Release 说明中添加：

```markdown
### macOS 安装指引

由于应用尚未经过苹果公证，首次打开时可能提示"已损坏"。请使用以下方法之一：

**方法 1（推荐）**：
\`\`\`bash
xattr -cr /Applications/ChangYan.app
\`\`\`

**方法 2**：
1. 尝试打开应用（会报错）
2. 打开 `系统设置 → 隐私与安全性`
3. 在"安全性"部分点击"仍要打开"
```

## 参考资料

- [Tauri Code Signing Guide](https://v2.tauri.app/distribute/sign/macos/)
- [Apple Notarization Guide](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [tauri-action 文档](https://github.com/tauri-apps/tauri-action)