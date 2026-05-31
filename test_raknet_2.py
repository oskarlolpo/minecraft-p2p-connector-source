import socket
import time
import struct

def ping():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(2.0)
    
    # RakNet Unconnected Ping
    magic = b'\x00\xff\xff\x00\xfe\xfe\xfe\xfe\xfd\xfd\xfd\xfd\x12\x34\x56\x78'
    req = b'\x01' + struct.pack('>Q', int(time.time())) + magic + struct.pack('>Q', 0)
    
    print('Sending UDP ping to 127.0.0.1:19132...')
    try:
        sock.sendto(req, ('127.0.0.1', 19132))
        data, addr = sock.recvfrom(1024)
        print(f'Received {len(data)} bytes from {addr}:')
        print(data.hex())
        if len(data) >= 35:
            string_len = struct.unpack('>H', data[33:35])[0]
            if len(data) >= 35 + string_len:
                server_id = data[35:35+string_len].decode('utf-8', errors='ignore')
                print('Server ID:', server_id)
    except Exception as e:
        print('Error:', e)

ping()
