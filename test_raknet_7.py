import socket
import time
import struct

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
sock.settimeout(2.0)
magic = b'\x00\xff\xff\x00\xfe\xfe\xfe\xfe\xfd\xfd\xfd\xfd\x12\x34\x56\x78'
req = b'\x01' + struct.pack('>Q', int(time.time())) + magic + struct.pack('>Q', 0)

for ip in ['192.168.31.124', '127.0.0.1', '255.255.255.255']:
    print(f'Sending UDP ping to {ip}:7551...')
    try:
        sock.sendto(req, (ip, 7551))
    except Exception as e:
        print('Error:', e)

try:
    while True:
        data, addr = sock.recvfrom(1024)
        print(f'Received {len(data)} bytes from {addr}:')
        if len(data) >= 35 and data[0] == 0x1c:
            string_len = struct.unpack('>H', data[33:35])[0]
            if len(data) >= 35 + string_len:
                server_id = data[35:35+string_len].decode('utf-8', errors='ignore')
                print('Server ID:', server_id)
        else:
            print(data.hex())
except Exception as e:
    print('Done waiting:', e)
