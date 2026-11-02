# ctdozlite — security hardening fork

Bản fork của [ctdoteam1110/ctdozlite](https://github.com/ctdoteam1110/ctdozlite), một Zalo client không chính thức viết bằng Rust với GUI và TUI. Giữ nguyên giấy phép MIT / Apache-2.0 và ghi nhận tác giả gốc.

**Phiên bản mã nguồn của fork: 0.1.4.** Các bản EXE/DEB/DMG từ repository gốc không chứa những sửa đổi dưới đây. Không dùng chúng để đánh giá bản fork.

## Bản 0.1.4 — giảm tăng RAM khi mở lịch sử lớn

- Chỉ tải ảnh/sticker gần vùng đang xem; bộ đệm texture giữ tối đa 64 MiB / 256 ảnh và loại ảnh ít dùng. Đây là giới hạn bộ đệm ảnh, không phải tổng RAM ứng dụng.
- Tối đa 3 lượt tải/giải mã ảnh; giữ giới hạn cả khi ảnh đã giải mã đang chờ giao diện. Ảnh trong hội thoại thu nhỏ tối đa 640px; mở ảnh lớn vẫn dùng bản 1600px. Không giải mã lại cùng tệp ba lần.
- Lưu lịch sử bằng một luồng duy nhất, gộp thay đổi trong mỗi khung hình và thay snapshot chờ bằng bản mới nhất; không tạo thêm một tác vụ ghi toàn bộ lịch sử cho từng sự kiện.
- Ánh xạ đồng bộ điện thoại chỉ giữ metadata hội thoại. Xác thực cache cũ khi lưu không dựng thêm toàn bộ lịch sử vào RAM. Hàng đợi khung hình cuộc gọi cũng được giới hạn.

## Bản 0.1.3

- Hiển thị giờ Việt Nam (GMT+7), sửa cả giờ trong cache cũ; hội thoại chưa có tin nhắn không hiện giờ giả.
- Lấy tên nhóm theo lô và giữ tên thật bắt đầu bằng “Nhóm”. Kiểm tra đọc trực tiếp đã nhận đủ tên của 215/215 nhóm trên tài khoản thử.
- Hỗ trợ IME trên Linux với bản vá egui-winit; đã thử nhập “tiếng việt” bằng IBus Bamboo trên X11. Enter chốt dấu không gửi tin; Enter tiếp theo mới gửi.
- Tự tải lịch sử còn trên máy chủ cho nhóm và Cloud, có tiến độ, dừng/thử lại và lưu từng trang vào kho mã hóa. Tin cũ không tăng số chưa đọc. Máy chủ có thể từ chối các trang cũ; giao diện báo số hội thoại chưa tải được.
- Đồng bộ trực tiếp từ Android qua sync-v2: dùng kết nối socket hiện có, xác nhận trên điện thoại, giải mã từng lô và lưu vào kho lịch sử mã hóa. Lần thử thật đã nhận/lưu 6.148 tin trước khi gặp tranh chấp SQLite do nhiều cửa sổ app cùng mở. Chưa coi toàn bộ lịch sử đã đồng bộ xong.
- Nút X thoát ứng dụng; chỉ mở một cửa sổ cho mỗi thư mục dữ liệu. Khi bấm mở lần nữa, đưa cửa sổ đang chạy lên trước.
- Ảnh hiển thị trong hội thoại, video có khung phát, tạm dừng/phát tiếp và âm thanh; tệp video tải tạm được xóa khi dừng/đóng. Cần FFmpeg và FFplay trên PATH; giới hạn video 256 MB.
- Ghi lịch sử theo giao dịch SQLite cho mỗi lô để tránh ghi dở dang và giảm tranh chấp.

Đồng bộ Android cần Node.js 24+ để chạy adapter giải mã riêng qua pipe; không dùng Electron. Thành phần JS/WASM công khai của Zalo được tải vào `phone_sync_runtime`, kiểm tra SHA256 trước khi dùng, không phân phối trong repo/bộ cài. Khóa chuyển lịch sử chỉ tồn tại trong bộ nhớ của helper; không ghi khóa vào cache hay log. Đây là ngoại lệ tùy chọn cho mục tiêu client Rust gọn nhẹ. Tài khoản hiện tại đã thử được metadata, xác nhận Android và nhập tin thật; còn cần thử việc hoàn tất toàn bộ và các trường hợp mất kết nối.

## Đã cải tiến

- Mã hóa `session.json` bằng AES-256-GCM với nonce ngẫu nhiên cho mỗi lần ghi. Khóa ngẫu nhiên 256-bit nằm trong kho khóa hệ điều hành, không nằm cạnh tệp phiên.
- Dùng Windows Credential Manager, macOS Keychain hoặc Linux Secret Service. Nếu kho khóa không hoạt động, không lưu phiên dạng văn bản; phiên hiện tại có thể tiếp tục trong RAM. Giao diện báo cần đăng nhập lại lần sau.
- Đặt dữ liệu vào thư mục riêng của người dùng; trên Unix giới hạn thư mục `0700`, tệp phiên `0600`. Ghi phiên và JSON cache bằng thay thế nguyên tử; có khóa tệp để điều phối nhiều tiến trình.
- Tắt log tín hiệu cuộc gọi, log VoIP và nội dung request/response mặc định, kể cả khi chỉ bật `--dev`. Không để lộ tiền tố cookie hoặc khóa trong hàm che bí mật.
- Chỉ gửi các request xác thực và tải tệp qua HTTPS tới `zalo.me`, `zaloapp.com` hoặc tên miền con đúng ranh giới; WebSocket phải dùng WSS. Chặn userinfo, cổng khác 443 và redirect của HTTP client có cookie.
- Mã hóa AES-256-GCM các trường nhạy cảm trong SQLite (tin nhắn, người gửi, tên hội thoại, dữ liệu trích dẫn/liên kết) và toàn bộ cache hội thoại. Khóa lịch sử tách khỏi khóa phiên, lưu trong kho khóa hệ điều hành. Mỗi trường gắn với ID và tên cột để phát hiện việc tráo dữ liệu.
- Tự chuyển lịch sử cũ trong giao dịch SQLite, dọn trang trống sau chuyển đổi và giữ nguyên dữ liệu khi chuyển lỗi. Không ghi đè lịch sử không giải mã được. ID, thời gian, số lượng và cấu trúc SQLite vẫn nhìn thấy; ảnh, sticker và tệp đính kèm chưa mã hóa. Xem [thiết kế mã hóa lịch sử](docs/HISTORY_ENCRYPTION.md).
- Đồng bộ phiên bản gói, ghim GitHub Actions theo commit và tạo chứng thực nguồn build cho các bản phát hành tương lai.

## Build

Cần Rust stable, CMake, trình biên dịch C/C++, pkg-config; trên Linux cần các gói phát triển ALSA/X11/XKB. FFmpeg chỉ cần khi gọi video. Ví dụ Ubuntu:

```bash
sudo apt install build-essential cmake pkg-config libasound2-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev \
  libssl-dev libgtk-3-dev zenity xclip wl-clipboard ffmpeg

git clone https://github.com/Trikim7/ctdozlite.git
cd ctdozlite
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo build --release --locked --bin zalo-rust
./target/release/zalo-rust --gui
```

Trên Windows dùng Visual Studio Build Tools (Desktop development with C++), CMake và `cargo build --release --locked --bin zalo-rust`. Trên macOS dùng Xcode Command Line Tools, CMake và pkg-config.

```bash
cargo test --locked
cargo run --locked --bin zalo-rust -- --version
```

Kiểm thử tích hợp kho khóa thật là opt-in, tạo rồi xóa một entry thử nghiệm; không đăng nhập Zalo:

```bash
cargo test --locked --lib native_vault_roundtrip -- --ignored --exact secure_session::tests::native_vault_roundtrip
```

## Dữ liệu và chuyển từ bản gốc

Ứng dụng in đường dẫn dữ liệu khi khởi động. Mặc định trên Linux là `$XDG_DATA_HOME/ctdozlite` hoặc `~/.local/share/ctdozlite`; trên Windows và macOS dùng đường dẫn dữ liệu riêng do hệ điều hành cung cấp.

Đăng nhập QR lại là cách chuyển phiên được khuyến nghị. App không tự quét hay sao chép dữ liệu trong thư mục cũ. Nếu chủ động sao chép `session.json` cũ vào thư mục dữ liệu mới, loader chỉ chấp nhận phiên sau khi đã thay tệp gốc bằng bản mã hóa thành công. Khi kho khóa bị khóa, tệp cũ được giữ nguyên và không được dùng để khôi phục phiên. Việc thay tệp không bảo đảm xóa sạch bản văn bản khỏi SSD, bản sao lưu hoặc lịch sử đồng bộ.

Các công cụ chẩn đoán chạy từ terminal không đổi thư mục làm việc; phải chỉ định đúng tệp phiên hoặc chạy tại thư mục dữ liệu. Khóa gắn với đường dẫn tuyệt đối của tệp; không thể chuyển tệp mã hóa sang máy hoặc thư mục khác rồi dùng ngay.

## Chẩn đoán nhạy cảm

`--dev` chỉ bật giao diện chẩn đoán với nội dung nhạy cảm bị ẩn. `--dump` / `--dump-raw` bị từ chối trừ khi có lựa chọn riêng:

```bash
CTDOZLITE_SENSITIVE_LOGS=1 ./target/release/zalo-rust --gui --dev
```

Chỉ dùng lựa chọn này với tài khoản thử nghiệm. Nó cho phép các đường log tín hiệu/VoIP; không gửi log, database hay tệp phiên lên GitHub. Dùng `ZALO_VOIP_LOG=off` để tắt riêng log gói VoIP.

## Giới hạn còn lại

Đây là một đợt cải tiến bảo mật, không phải chứng nhận app an toàn. Chưa kiểm chứng tính tương thích với Zalo thật, cuộc gọi, camera/microphone hay tính ổn định của giao thức. Mã hóa phiên không bảo vệ trước mã độc chạy cùng tài khoản hệ điều hành. Bản fork không bổ sung tính năng tự cập nhật chưa có trong nguồn công khai.

Xem [SESSION_SECURITY.md](SESSION_SECURITY.md) và [docs/SECURITY_HARDENING.md](docs/SECURITY_HARDENING.md) để biết thiết kế, kiểm thử và các cảnh báo thư viện còn lại.
