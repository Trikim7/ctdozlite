# Giao diện và thành phần cuộc gọi — 0.1.5

## Đối chiếu bằng REA

Đã dùng REA để phân tích tĩnh các tệp Zalo Web được tải trong phiên nghiên cứu trước (`web-shared.js`, `web-startup.js`, bootstrap `chat.html`). Bundle lớn chạm giới hạn heap 384 MiB; không tăng giới hạn hoặc coi đó là phân tích toàn bộ ứng dụng. Sau đó trích nguyên các factory O7EA, 7UAy và Gm1y thành các tệp nhỏ, kiểm tra cú pháp mà không chạy chúng, giữ SHA256 tệp gốc và khoảng UTF-16 trong provenance; REA phân tích lại phạm vi nhỏ này.

Những quan sát dùng cho thay đổi: settings có `setting-menu` và `setting-content`, danh sách mục Cài đặt chung/Quyền riêng tư/Đồng bộ/Giao diện/Thông báo/Tin nhắn/Tiện ích, chế độ compact khi cửa sổ hẹp, chiều cao phụ thuộc viewport, và nền tối `#22262B` trong bootstrap. Group profile dùng `avt` cho avatar và `fullAvt` cho ảnh nền. Đây là bằng chứng cấu trúc tĩnh của Web; chưa phải đối chiếu từng pixel với client Windows hoặc bằng chứng toàn bộ hành vi settings gốc.

Evidence thu hẹp: `ev_2028e924b219edfa75f96e964843747c0af153d41115c11d818cc537d38a254a`. Source bundle SHA256: `f1f574c135e0016d0896910d12b7886cf65cb6b791bd9e813d4d7642e662fcba`. Các quan hệ động, module phụ thuộc và phần semantic vượt hạn mức vẫn chưa biết. Không phân phối code Zalo hoặc runtime REA trong app.

## Các lỗi giao diện đã sửa

- Khung cũ cộng chiều rộng sidebar, nội dung và margin hai lần, ép chiều cao tối thiểu lớn hơn cửa sổ nhỏ. Khung mới tính một hình chữ nhật theo viewport, header riêng, sidebar/content được giới hạn và cuộn riêng; compact dùng danh sách chọn mục.
- Các thẻ xem trước giao diện/thông báo cũ dùng `vertical_centered` ăn hết chiều rộng của mỗi ô. Mỗi thẻ nay có kích thước riêng, có thể xuống dòng thay vì làm cửa sổ nở ngang.
- Avatar cũ giới hạn kích thước vùng crop trước khi resize, làm ảnh lớn bị cắt thành một mảnh nhỏ ở giữa. Nay lấy toàn bộ hình vuông ở giữa rồi resize 128px và tạo alpha tròn; ảnh vuông không mất phần ngoài khuôn mặt, ảnh nhỏ không bị phép trừ underflow. Avatar lỗi có thời gian chờ retry và cache hỏng được bỏ để tải lại.
- Các mục quyền riêng tư tài khoản và PIN vốn chỉ lưu biến cục bộ chưa có chức năng thực thi được hiển thị chưa hỗ trợ/khóa điều khiển, tránh khiến người dùng nghĩ tài khoản hoặc cuộc trò chuyện đã được bảo vệ bởi các công tắc đó.

## Thành phần gọi điện

Thoại dùng CPAL/ALSA và Opus đã được biên dịch cùng client. Video cần FFmpeg hỗ trợ libx264/H.264 và backend camera tương ứng (v4l2 trên Linux, dshow trên Windows, avfoundation trên macOS). FFplay phục vụ âm thanh video đính kèm, không phải backend thoại VoIP.

`call_check` kiểm tra cấu hình thiết bị mặc định, Opus encode/decode với tín hiệu giả, encoder/decoder của FFmpeg và danh sách camera. Không gọi sang tài khoản khác, không ghi âm hoặc quay camera. GUI thực hiện cùng phép kiểm tra theo yêu cầu, có nút kiểm tra lại, lựa chọn loa/tai nghe cho cuộc gọi tiếp theo. Danh sách thiết bị được cache thay vì quét mỗi frame.

Trên máy thử, các thành phần trên đã có từ cài đặt trước và phép kiểm tra đều đạt; không cài thêm gói hệ thống không cần thiết. Đã chạy fixture H.264 bằng FFmpeg và bài thử Opus roundtrip. Điều này chưa chứng minh cuộc gọi Zalo đầu-cuối, chất lượng âm thanh, webcam thực tế hoặc khả năng xuyên NAT của giao thức.

## Tài nguyên và xác minh

Giữ giới hạn texture 64 MiB/256 ảnh, 3 tác vụ ảnh có backpressure qua GUI, writer lịch sử gộp snapshot của 0.1.4. Bỏ việc đọc và gộp lại toàn bộ cache lần hai khi khởi động; SQLite được hydrate một lần. Không thêm dependency Rust, framework giao diện, dịch vụ hoặc runtime nền mới.

133 bài kiểm thử thư viện đạt (4 bài bỏ qua mặc định), gồm crop ảnh nhỏ/lớn, geometry cài đặt 480×360/1020×700/1920×1080, Opus roundtrip và các giới hạn bộ nhớ đã có. Release Linux build đạt. Đã kiểm tra giao diện thật các tab, các ô xem trước và cửa sổ nhỏ; ảnh chụp và phép đo RAM chỉ lưu ở thư mục làm việc riêng, không đưa dữ liệu trò chuyện lên repo.
