# E2E environment lock contract v1

Phase 0a freezes this lock *format*, not a speculative package digest. The
locked Rust capability runner validates the environment before it enables a
case, golden, or CI job. A missing or mutable field is an error.

```toml
schema_version = 1
platform = "linux/amd64"
base_image = "docker.io/library/rust@sha256:<64 lowercase hex>"
containerfile_sha256 = "<64 lowercase hex>"
apt_snapshot = "https://snapshot.debian.org/archive/<suite>/<UTC timestamp>/"
packages = [{ name = "exact package name", version = "exact Debian version" }]
fonts = [{ path = "/usr/share/fonts/...", sha256 = "<64 lowercase hex>" }]
sway_command = ["sway", "--unsupported-gpu", "--config", "/workspace/tools/ui-testing/sway.conf"]
dbus_command = ["dbus-run-session", "--"]
renderer_env = { EGL_PLATFORM = "wayland", LIBGL_ALWAYS_SOFTWARE = "1", WGPU_BACKEND = "vulkan", WLR_BACKENDS = "headless", WLR_LIBINPUT_NO_DEVICES = "1", WLR_RENDERER = "pixman", LIBSEAT_BACKEND = "noop" }
home = "/tmp/ui-test-home"
xdg_config_home = "/tmp/ui-test-config"
xdg_cache_home = "/tmp/ui-test-cache"
locale = "C.UTF-8"
locale_env = { LANG = "C.UTF-8", LC_ALL = "C.UTF-8", TZ = "UTC" }
fontconfig_file = "/etc/fonts/fonts.conf"
cosmic_theme_fixture = "/opt/ui-test/cosmic-theme.toml"
theme = "light"
viewport = { width = 1280, height = 800, scale = 1 }
atspi_coordinate_space = "logical_output_origin_0_0"
image_comparison = { colorspace = "sRGB RGBA", channel_delta = 2, max_changed_pixel_fraction = "0.0005", masks = false, edge_rounding = "floor_start_ceil_end" }
capture_helper = { name = "grim", version = "exact Debian version", protocol = "zwlr_screencopy_manager_v1" }
input_helper = { name = "wtype", version = "exact Debian version", protocol = "zwp_virtual_keyboard_manager_v1" }
toolchain = { rust = "exact rust-toolchain.toml version", app_built_in_image = true }
```

The lock records the OCI index digest for the selected platform and the exact
package/font bytes actually installed in the built image. It is refreshed only
by an explicit `just ui-e2e-update-environment` command that produces an image
diff, package diff, font hashes, and new goldens in the same reviewed change.
`just ui-e2e`, CI, and golden updates reject a tag-only base image, an unpinned
APT source, a missing lock hash, a host compositor, a system D-Bus connection,
or an environment differing from the lock. The runner creates the three XDG
directories, exports the locked locale/timezone/Fontconfig values, and installs
the theme fixture before it starts headless Sway. AT-SPI bounds are logical coordinates
on the sole output and are transformed to pixels only by the locked
`edge_rounding` rule.
