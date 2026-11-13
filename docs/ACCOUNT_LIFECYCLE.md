# Đăng nhập, đăng xuất và độ ổn định — 0.1.8

## Sử dụng

Mở ảnh đại diện → **Đăng xuất / Đổi tài khoản**, hoặc **Cài đặt → Cài đặt chung → Tài khoản**. App đóng phiên trên máy rồi mở lại màn hình đăng nhập. Quét QR bằng Zalo điện thoại và xác nhận; màn hình số điện thoại/mật khẩu vẫn có nếu máy chủ hỗ trợ tài khoản đó.

**Giữ đăng nhập khi đóng ứng dụng** được áp dụng khi thoát app bình thường: tắt tùy chọn sẽ xóa phiên đăng nhập trên máy sau khi cửa sổ đóng, giữ lịch sử mã hóa. Tùy chọn này không thay đổi phiên trên thiết bị khác. Khi đăng nhập/đăng xuất, cửa sổ được mở lại một lần để khởi tạo sạch kết nối và tác vụ nền.

## Dữ liệu tài khoản

- Tài khoản cũ tiếp tục dùng đúng đường dẫn dữ liệu và khóa trong kho hệ điều hành, không di chuyển hoặc mã hóa lại lịch sử cũ.
- Metadata chỉ lưu SHA-256 định danh tài khoản. Các tài khoản khác có thư mục riêng `accounts/<sha256>`, phiên và khóa riêng; không nạp lịch sử của tài khoản trước vào tài khoản mới.
- Đăng xuất chỉ xóa `session.json`, giữ cache và SQLite. Việc xóa phiên được đồng bộ với bộ ghi phiên; tác vụ đang chờ ghi không thể tạo lại phiên cũ sau đăng xuất.
- Đăng nhập mới chỉ được lưu vào thư mục đúng tài khoản sau khi nhận đủ thông tin phiên. Con trỏ tài khoản được ghi nguyên tử. Dữ liệu cũ không xác định được chủ sở hữu được giữ nguyên, không tự gán sang người vừa đăng nhập.
- Toàn ứng dụng dùng một khóa phiên bản đang chạy, kể cả khi thư mục tài khoản thay đổi. Tiến trình mới chờ tiến trình cũ thoát, không để socket/polling/upload của tài khoản cũ chạy trong tài khoản mới.

Không di chuyển thủ công tệp mã hóa. Các bản trước 0.1.8 không hiểu lựa chọn thư mục tài khoản mới.

## Ổn định

- Chỉ lỗi hết phiên rõ ràng từ máy chủ mới yêu cầu đăng nhập lại. Timeout, mất mạng, HTTP 5xx không tự xóa phiên.
- Chặn các lượt làm mới danh bạ chạy chồng. Lỗi tải danh bạ giữ dữ liệu hiện có.
- Cờ polling/làm mới được nhả bằng RAII khi hoàn tất, hủy hoặc unwind; không kẹt sau khi một tác vụ bị dừng.
- Mỗi khung hình xử lý tối đa 64 sự kiện để cửa sổ còn phản hồi khi dữ liệu về dồn.
- QR/mật khẩu dùng phiên tạm, không giữ khóa session chung trong suốt thời gian chờ điện thoại hoặc mạng. Tạo QR mới/hình thức đăng nhập mới hủy tác vụ đăng nhập trước.
- Báo rõ QR hết hạn/lỗi kết nối, không đưa URL chứa tham số xác thực ra thông báo. Mật khẩu được chuyển khỏi ô nhập và xóa vùng đệm khi tác vụ kết thúc.
- Giữ các giới hạn ảnh, upload và writer lịch sử hiện có; không thêm dependency/runtime nền.

## Kiểm thử và giới hạn

152 test thư viện đạt, 5 test ignore mặc định. Đã chạy riêng test chu trình mã hóa A → đăng xuất → A → B → A bằng tài khoản giả, dữ liệu và khóa tạm trong kho khóa thật; kiểm tra lịch sử không lẫn, đọc lại được và phiên không bị ghi lại sau đăng xuất. Test chỉ chạy trên Linux, yêu cầu thư mục fixture có marker riêng và dọn khóa thử sau khi chạy.

Native X11/Xwayland: mở Cài đặt, bấm Đăng xuất trên tài khoản giả; tiến trình cũ thoát, tiến trình mới mở màn hình đăng nhập, tệp phiên bị xóa và lịch sử mã hóa vẫn đọc được. Chạy với mạng bị chặn cũng giữ được phiên trước khi chủ động đăng xuất. Đã thử riêng đóng cửa sổ với tùy chọn Giữ đăng nhập tắt: phiên bị xóa, lịch sử vẫn giải mã được và không mở lại tiến trình. Thử này không đăng xuất tài khoản thật, không gửi tin nhắn/file, không yêu cầu đồng bộ Android và không gọi điện.

Đăng nhập bằng quét QR thật và đổi sang tài khoản Zalo thật thứ hai cần người dùng quét/xác nhận trên điện thoại; chưa được coi đã kiểm chứng đầu cuối trong đợt này. Không coi kiểm thử ngắn là chứng minh RAM ổn định dài hạn hoặc mọi lỗi hệ thống đã được xử lý.
