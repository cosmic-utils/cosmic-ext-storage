app-title = 存储
settings = 设置
about = 关于

git-description = Git 提交 {$hash} 于 {$date}

# Menu items
new-disk-image = 新建磁盘映像
attach-disk-image = 挂载磁盘映像
create-disk-from-drive = 从驱动器创建磁盘映像
create-image = 创建映像
restore-image-to-drive = 将映像恢复到驱动器
restore-image = 恢复映像
create-disk-from-partition = 从分区创建磁盘映像
restore-image-to-partition = 将磁盘映像恢复到分区
image-file-path = 映像文件路径
image-destination-path = 目标文件路径
image-source-path = 源映像路径
image-size = 映像大小
choose-path = 选择...
no-file-selected = 未选择文件
attach = 挂载
restore-warning = 这将覆盖选定的目标设备。此操作无法撤销。
eject = 弹出
eject-failed = 弹出失败
power-off = 关闭电源
power-off-failed = 关闭电源失败
format-disk = 格式化磁盘
format-disk-failed = 格式化磁盘失败
smart-data-self-tests = SMART 数据与自检
standby-now = 立即待机
standby-failed = 待机失败
wake-up-from-standby = 从待机唤醒
wake-up-failed = 唤醒失败
unmount-failed = 卸载失败

# Unmount busy dialog
unmount-busy-title-template = {$device} 正忙
unmount-busy-message-template = 以下进程正在访问 {$mount}
unmount-busy-header-pid = 进程号
unmount-busy-header-command = 命令
unmount-busy-header-user = 用户
unmount-busy-no-processes = 设备正忙但未找到进程。请重试或手动关闭所有文件。
unmount-busy-kill-warning = 终止进程可能导致数据丢失或损坏。
unmount-busy-kill-and-retry = 终止进程并重试
retry = 重试

# Dialog buttons
ok = 确定
cancel = 取消
continue = 继续
working = 处理中…

# Common
close = 关闭
refresh = 刷新
next = 下一步
behavior = 行为
credentials = 凭据
review = 审核
details = 详情

# Format disk dialog
erase-dont-overwrite-quick = 不覆写（快）
erase-overwrite-slow = 覆写（慢）
partitioning-dos-mbr = 传统兼容（DOS/MBR）
partitioning-gpt = 现代（GPT）
partitioning-none = 无

# Create partition dialog
create-partition = 创建分区
create-partition-failed = 创建分区失败
format-partition = 格式化分区
format = 格式化
format-partition-description = 这将格式化选定的卷。大小：{ $size }
volume-name = 卷名称
partition-name = 分区名称
partition-size = 分区大小
free-space = 可用空间
erase = 擦除
password-protected = 密码保护
password = 密码
confirm = 确认
password-required = 需要密码。
password-mismatch = 密码不匹配。
apply = 应用
untitled = 未命名

# Main view
no-disk-selected = 未选择磁盘
no-volumes = 无可用卷
partition-number = 分区 { $number }
partition-number-with-name = 分区 { $number }：{ $name }
volumes = 卷
unknown = 未知
unresolved = 未解析

# Info labels
size = 大小
usage = 使用情况
mounted-at = 挂载于
contents = 内容
device = 设备
partition = 分区
path = 路径
uuid = UUID
model = 型号
serial = 序列号
partitioning = 分区方案
backing-file = 后备文件

# Confirmation dialog
delete = 删除 { $name }
delete-partition = 删除
delete-confirmation = 您确定要删除 { $name } 吗？
delete-failed = 删除失败

# Volume segments
free-space-segment = 可用空间
reserved-space-segment = 保留
filesystem = 文件系统
free-space-caption = 可用空间
reserved-space-caption = 保留空间

# Encrypted / LUKS
unlock-button = 解锁
lock = 锁定
unlock = 解锁 { $name }
passphrase = 密钥
current-passphrase = 当前密钥
new-passphrase = 新密钥
change-passphrase = 更改密钥
passphrase-mismatch = 密钥不匹配。
locked = 已锁定
unlocked = 已解锁
unlock-failed = 解锁失败
lock-failed = 锁定失败
unlock-missing-partition = 在当前设备列表中找不到 { $name }。

# Volume commands
mount = 挂载
unmount = 卸载
operation-cancelled = 操作已取消
edit-mount-options = 编辑挂载选项…
edit-mount-options-failed = 编辑挂载选项失败
edit-encryption-options = 编辑加密选项…
edit-partition = 编辑分区
edit = 编辑
edit-partition-no-types = 此分区表没有可用的分区类型。
flag-legacy-bios-bootable = 传统 BIOS 可启动
flag-system-partition = 系统分区
flag-hide-from-firmware = 对固件隐藏
resize-partition = 调整分区大小
resize = 调整大小
resize-partition-range = 允许范围：{ $min } 到 { $max }
new-size = 新大小
edit-filesystem = 编辑文件系统
label = 标签
filesystem-label = 文件系统标签
check-filesystem = 检查文件系统
check-filesystem-warning = 检查文件系统可能需要很长时间。是否继续？
repair-filesystem = 修复文件系统
repair = 修复
repair-filesystem-warning = 修复文件系统可能需要很长时间，并可能导致数据丢失。是否继续？
take-ownership = 获取所有权
take-ownership-warning = 这将把文件的所有权更改为您的用户。这可能需要很长时间，且无法轻易撤销。
take-ownership-recursive = 递归应用

# Mount/encryption options
user-session-defaults = 用户会话默认值
mount-at-startup = 系统启动时挂载
unlock-at-startup = 系统启动时解锁
require-auth-to-mount = 挂载或卸载需要授权
require-auth-to-unlock = 解锁需要授权
show-in-ui = 在用户界面中显示
identify-as = 标识为
other-options = 其他选项
mount-point = 挂载点
filesystem-type = 文件系统类型
display-name = 显示名称
icon-name = 图标名称
symbolic-icon-name = 符号图标名称
show-passphrase = 显示密钥
name = 名称

# SMART
smart-no-data = 没有可用的 SMART 数据。
smart-type = 类型
smart-updated = 更新于
smart-temperature = 温度
smart-power-on-hours = 通电时间
smart-selftest = 自检
smart-selftest-short = 短自检
smart-selftest-extended = 扩展自检
smart-selftest-abort = 中止自检

# Volume types
lvm-logical-volume = LVM 逻辑卷
lvm-physical-volume = LVM 物理卷
luks-container = LUKS
partition-type = 分区
block-device = 设备

# Status
not-mounted = 未挂载
can-create-partition = 可创建分区

# Filesystem tools detection
fs-tools-missing-title = 缺少文件系统工具
fs-tools-missing-desc = 以下工具未安装。请安装它们以启用完整的文件系统支持：
fs-tools-all-installed-title = 文件系统工具
fs-tools-all-installed = 所有文件系统工具已安装。
fs-tools-required-for = 需要 {$fs_name} 支持

# UDisks2 BTRFS module
settings-enable-ustorage-btrfs = 尝试启用 UDisks2 BTRFS
settings-ustorage-btrfs-enabled = UDisks2 BTRFS 模块已启用
settings-ustorage-btrfs-enabled-body = UDisks2 BTRFS 模块已成功启用。您现在可以使用 BTRFS 管理功能。
settings-ustorage-btrfs-failed = 启用 UDisks2 BTRFS 模块失败

offset = 偏移量

# Partition dialog labels
overwrite-data-slow = 覆写数据（慢）
password-protected-luks = 密码保护（LUKS）

# Filesystem type names
fs-name-ext4 = ext4
fs-name-ext3 = ext3
fs-name-xfs = XFS
fs-name-btrfs = Btrfs
fs-name-f2fs = F2FS
fs-name-udf = UDF
fs-name-ntfs = NTFS
fs-name-vfat = FAT32
fs-name-exfat = exFAT
fs-name-swap = 交换分区

# Filesystem type descriptions
fs-desc-ext4 = 现代 Linux 文件系统（默认）
fs-desc-ext3 = 传统 Linux 文件系统
fs-desc-xfs = 高性能日志文件系统
fs-desc-btrfs = 支持快照的写时复制文件系统
fs-desc-f2fs = 针对闪存优化的文件系统
fs-desc-udf = 通用磁盘格式
fs-desc-ntfs = Windows 文件系统
fs-desc-vfat = 通用兼容性
fs-desc-exfat = 支持大文件，跨平台
fs-desc-swap = 虚拟内存

# Filesystem tools warning
fs-tools-warning = 由于缺少工具，某些文件系统类型不可用。请参阅设置了解更多信息。

# Detail Tabs
volume-info = 卷信息

# BTRFS Management
btrfs-management = BTRFS 管理
btrfs = BTRFS
volume = 卷
btrfs-placeholder = BTRFS 管理功能即将推出
btrfs-create-subvolume = 创建子卷
btrfs-subvolume-name = 子卷名称
btrfs-subvolume-name-required = 子卷名称为必填项
btrfs-subvolume-invalid-chars = 子卷名称不能包含斜杠
btrfs-create-subvolume-failed = 创建子卷失败
btrfs-delete-subvolume = 删除子卷
btrfs-delete-confirm = 删除子卷「{ $name }」？此操作无法撤销。
btrfs-delete-subvolume-failed = 删除子卷失败
btrfs-create-snapshot = 创建快照
btrfs-source-subvolume = 源子卷
btrfs-snapshot-name = 快照名称
btrfs-read-only = 只读快照
btrfs-create-snapshot-failed = 创建快照失败
btrfs-used-space = 已用空间
btrfs-subvolume-id = ID
btrfs-subvolume-path = 路径
btrfs-subvolume-actions = 操作
btrfs-set-default-failed = 设置默认子卷失败
btrfs-readonly-failed = 切换只读标志失败
btrfs-not-mounted = BTRFS 文件系统未挂载
btrfs-not-mounted-refresh = BTRFS 文件系统未挂载（请尝试刷新）
btrfs-loading-subvolumes = 正在加载子卷...
btrfs-no-subvolumes = 未找到子卷
btrfs-no-subvolumes-desc = 此 BTRFS 卷可能是新创建的，或者尚未包含任何子卷。
btrfs-loading-usage = 正在加载使用信息...
btrfs-usage-error = 使用错误：{ $error }

# Usage view
usage-scanning = 正在扫描磁盘使用情况...
usage-scan-failed = 使用情况扫描失败
usage-scan-not-started = 使用情况扫描未开始
usage-files-per-category = 各类别文件数
usage-filename = 文件名
usage-selected-count = 已选择：{ $count }
usage-clear-selection = 清除选择
usage-configure = 配置
usage-show-all-root-mode = 显示所有文件（Root 模式）
usage-scan-setup = 使用扫描设置
usage-choose-mount-points = 选择挂载点
usage-choose-mount-points-desc = 选择一个或多个要包含在扫描中的挂载点。
usage-scan-parallelism-label = 并行扫描
usage-parallelism-low = 低
usage-parallelism-balanced = 平衡
usage-parallelism-high = 高
usage-selected = 已选择
usage-not-selected = 未选择
usage-loading-mount-points = 正在加载挂载点...
usage-no-mount-points = 没有可用的挂载点。
usage-parallelism = 并行
usage-start-scan = 开始扫描
usage-select-at-least-one-mount-point = 请至少选择一个挂载点
usage-delete-summary = 已删除 { $deleted } 个文件；{ $failed } 个失败

# Usage categories
usage-category-documents = 文档
usage-category-images = 图片
usage-category-audio = 音频
usage-category-video = 视频
usage-category-archives = 归档
usage-category-code = 代码
usage-category-binaries = 二进制文件
usage-category-packages = 软件包
usage-category-system = 系统
usage-category-other = 其他
