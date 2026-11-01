# Tiến độ ctdozlite — 01/11/2026

## Yêu cầu

Tự tải tin nhắn cũ, ưu tiên đồng bộ trực tiếp từ điện thoại; hỗ trợ IBus Bamboo; lấy tên nhóm thật; hiển thị giờ GMT+7.

## Đã xác minh

- 215/215 nhóm lấy được tên, 79 avatar. Chẩn đoán chỉ in số lượng/trường dữ liệu, không in tên, ID, nội dung hoặc khóa.
- GMT+7 cho tin nhắn, hội thoại, giờ đọc SQLite và cache cũ. Xóa giờ giả của hội thoại không có tin; metadata mới có thể sửa giờ cũ.
- IBus Bamboo/X11 đã gõ đúng “tiếng việt” trong native probe ngoại tuyến. Enter đầu commit không gửi, Enter sau mới kích hoạt gửi giả. Không gửi tin thử vào tài khoản thật, không đổi cấu hình Bamboo toàn hệ thống.
- Vendor egui-winit có giấy phép và ghi nguồn upstream. Bổ sung xử lý XIM Disabled đến trước Commit và Commit không có Preedit để không mất từ thứ hai.
- Backfill nhóm/Cloud tự chạy một lần sau đăng nhập: phân trang đến hết, chặn cursor lặp, lưu từng trang mã hóa thay vì ghi lại cả hội thoại mỗi trang, giữ timestamp mới nhất trong SQLite, không báo unread. Có dừng/thử lại.
- Kiểm tra trực tiếp nhận được 25 tin ở trang gần nhất của một nhóm; yêu cầu trang cũ trả lỗi -69. Chưa xác nhận tải hết nhiều trang trên tài khoản thật; app báo thất bại thay vì coi đó là lịch sử trống.
- Bộ thử thư viện cuối: 118 pass, 0 fail, 3 ignore, gồm sửa SQLite. Release build thành công. Clippy với -D warnings chưa đạt vì 42 cảnh báo ở mã cũ; không coi lint là đạt.

## Đồng bộ điện thoại — còn dang dở

- Chưa gửi yêu cầu chuyển lịch sử thật vì receiver/importer chưa hoàn chỉnh.
- Đã xác minh ngoại tuyến bằng dữ liệu giả: crypto WASM chính thức tạo identity/ephemeral, mã hóa/giải mã metadata và message, kiểm tra chống sửa ciphertext.
- Đã thử trọn định dạng: protobuf -> Zstd -> mã hóa -> file chunk có u32 little-endian -> giải mã -> giải nén -> protobuf. Giữ đúng tiếng Việt và ID lớn. Đây không phải bằng chứng đồng bộ với điện thoại thật.
- Thành phần proprietary tải công khai chỉ nằm ngoài repo trong work/reference, không phân phối cùng app. Binary Rust không phụ thuộc Node.js.
- Còn cần runtime crypto được phép phân phối hoặc backend tương thích; nối request/receiver trên socket hiện có; quản lý xác nhận/cancel trên điện thoại; ánh xạ globalNoisedId với UID hội thoại; nhập dữ liệu thật vào kho mã hóa và thử trên thiết bị.
- Không đánh dấu yêu cầu đồng bộ điện thoại hoàn tất chỉ vì tải được lịch sử máy chủ.

## Điểm tiếp tục và dữ liệu

- Nhánh: wip/history-ibus-groups-vntime; commit lưu trước lần tiếp tục: 9aebef1.
- Mã nguồn: work/ctdozlite; build helper: work/build.sh (đường dẫn máy cụ thể ngoài repo).
- Dữ liệu thật: ~/.local/share/ctdozlite. Khóa gắn với đường dẫn tuyệt đối; giữ nguyên dữ liệu và kho khóa khi cài cập nhật.
- Chẩn đoán đọc tài khoản: work/diagnostics/check_sync.rs, đã chuyển ra ngoài repo.
- IME probe: src/bin/ime_probe.rs; script thao tác ngoại tuyến work/type_ime_probe.py.
- Nghiên cứu: work/reference/full-transfer-probe.cjs, crypto-probe.cjs, protobuf-probe.cjs và client JS/WASM công khai. Không đưa dữ liệu/khóa thật vào các probe này.
- Nhật ký: work/final-012-tests.log, work/final-012-clippy.log, work/release-012.log, work/live-pagination*.log.
