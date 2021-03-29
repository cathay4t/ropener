# Rust MIME Software Opener

Simple replacement of xdg-open.

Example:

```toml
[global]
default = "/usr/bin/xdg-open"
text_viewer = "term -e vim"
web = "web --new-tab"
file_manager = "vimfm"
video_viewer = "mpv --fullscreen --slang=chi,eng"
#video_viewer = "vlc"
pdf_viewer = "zathura"
image_viewer = "sxiv -s f"

[audio]
default = "video_viewer"
mp4 = "video_viewer"

[text]
default = "text_viewer"
html = "web"

[inode]
x-empty = "text_viewer"
directory = "file_manager"

[video]
default = "video_viewer"

[application]
pdf = "pdf_viewer"
octet-stream = "video_viewer"

[image]
default = "image_viewer"
svg = "web"
```
