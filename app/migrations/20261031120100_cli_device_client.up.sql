-- Names the program that started a browser CLI login, such as
-- `dopbase 0.2.0 on macOS`, so the approval page can show which client
-- asked next to its hostname. Empty for requests from older clients.
ALTER TABLE cli_device_requests ADD COLUMN client_name TEXT NULL;
