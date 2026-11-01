# Mã hóa lịch sử chat — 0.1.1

Bản fork dùng **mã hóa ở cấp trường dữ liệu** để giữ SQLite hiện có và tránh thêm SQLCipher hoặc yêu cầu nhập mật khẩu mỗi lần mở app.

## Phần được bảo vệ

Các cột `sender_id`, `sender_name`, `text`, `msg_type`, `raw_json` trong bảng `messages` và `name` trong bảng `threads` được mã hóa AES-256-GCM. Dữ liệu trích dẫn, phản hồi và xem trước liên kết nằm trong `raw_json`, vì vậy cũng được bảo vệ. Toàn bộ `threads_cache.json` được mã hóa, gồm tên, đoạn xem trước và các tin nhắn được cache; tên tệp được giữ để tương thích các đường đăng xuất hiện tại.

Mỗi lần mã hóa tạo nonce ngẫu nhiên 96-bit mới. Dữ liệu xác thực bổ sung gắn với phiên bản định dạng, tên trường và ID bản ghi. Tráo ciphertext giữa hai tin nhắn hoặc hai cột sẽ bị từ chối. Đây không phải cơ chế chống khôi phục một bản sao lưu cũ hợp lệ.

Mỗi tệp database/cache có khóa ngẫu nhiên 256-bit riêng trong kho khóa hệ điều hành. Các khóa này độc lập với khóa phiên đăng nhập. Tệp không chứa khóa giải mã và không có khóa cố định trong mã nguồn. Khóa được tra theo đường dẫn tuyệt đối: sao chép hoặc đổi vị trí tệp không tự chuyển khóa sang máy/thư mục mới.

## Chuyển dữ liệu cũ

Đóng các phiên bản client cũ trước khi mở 0.1.1. Lần đầu truy cập database cũ, ứng dụng lấy khóa từ kho hệ điều hành, mã hóa các hàng trong một giao dịch SQLite và ghi dấu phiên bản/kiểm tra khóa. Chỉ sau khi giao dịch thành công mới sử dụng lịch sử đã chuyển.

Nếu chuyển đổi bị lỗi trước khi commit, SQLite rollback và dữ liệu cũ vẫn còn. Nếu mất điện hoặc hết dung lượng trong bước dọn trang, dấu hoàn tất chưa được đặt; lần mở sau sẽ thử dọn lại. Không tự tạo một bản sao lưu văn bản mới. Dữ liệu cache cũ chỉ được thay bằng ciphertext sau khi đọc JSON và mã hóa thành công.

Sau chuyển đổi, ứng dụng dùng `secure_delete=ON`, chế độ journal DELETE và `VACUUM` để giảm phần văn bản cũ còn trong tệp SQLite đang dùng. Xem tài liệu SQLite về [secure_delete](https://www.sqlite.org/pragma.html#pragma_secure_delete) và [VACUUM](https://www.sqlite.org/lang_vacuum.html). Bước này có thể cần thêm dung lượng đĩa và không xóa được bản sao lưu, snapshot, lịch sử đồng bộ hoặc các khối SSD cũ.

Thiếu/sai khóa hoặc dữ liệu mã hóa bị sửa sẽ gây lỗi đọc. Database đã đánh dấu mã hóa không được tự nhận khóa thay thế. Cache không đọc được sẽ không bị ghi đè bằng một danh sách trống. App tiếp tục có thể giữ tin nhắn trong RAM, đồng thời hiển thị cảnh báo về lưu trữ lịch sử; không quay về lưu văn bản.

## Mức bảo vệ và giới hạn

Người chỉ có tệp database/cache mà không có khóa trong kho hệ điều hành sẽ không đọc được các trường nội dung bằng công cụ SQLite/soạn thảo văn bản thông thường.

Đây **không phải mã hóa toàn database**. Cấu trúc bảng, ID tin nhắn/hội thoại, thời gian, số lượng hàng, hướng gửi/nhận, cờ nhóm và kích thước nội dung vẫn có thể quan sát. Các trường metadata đó không được xác thực bằng lớp mã hóa này. Ảnh đại diện, ảnh/sticker cache và tệp tải xuống chưa được mã hóa. Khi chủ động bật log nhạy cảm, nội dung có thể xuất hiện trong log.

Mã hóa này không ngăn mã độc đang chạy cùng tài khoản hệ điều hành truy cập app/kho khóa đã mở. Mất khóa đồng nghĩa không thể đọc lịch sử cục bộ; đăng nhập QR lại chỉ khôi phục phiên, không khôi phục khóa lịch sử. Không xóa các entry của app khỏi kho khóa nếu còn cần dữ liệu.

## Kiểm thử

Kiểm thử tự động dùng dữ liệu giả để kiểm tra:

- Chuyển database cũ sang mã hóa và mở lại với cùng khóa, giữ đúng nội dung Unicode và dữ liệu trích dẫn.
- Ghi/cập nhật/đọc tin nhắn, sắp xếp và phục hồi tên hội thoại.
- Không thấy chuỗi bí mật thử nghiệm trong các tệp database/journal đang còn sau chuyển đổi.
- Rollback toàn bộ khi chèn lỗi giữa chừng, rồi chuyển lại thành công.
- Thử lại bước dọn trang sau khi dấu hoàn tất chưa được đặt.
- Phát hiện sai khóa, ciphertext hỏng, tráo trường/bản ghi và nonce không hợp lệ.
- Mã hóa toàn bộ cache để không lộ tên hoặc đoạn xem trước.

Bài kiểm thử `native_history_and_cache_roundtrip_and_missing_key_protection` đã chạy với kho khóa thật trên Linux: tạo rồi xóa khóa thử nghiệm, chuyển cache cũ, mở lại database/cache và xác nhận việc mất khóa không làm tạo khóa thay thế hoặc ghi đè dữ liệu cũ. Chưa thử bằng lịch sử/tài khoản Zalo thật hay kho khóa Windows/macOS.
