import socket
import time
import struct

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.settimeout(2.0)
target = ('192.168.31.124', 7551)

# RakNet Unconnected Ping
# ID (1) + Time (8) + Magic (16) + Client GUID (8)
packet = b'\x01' + struct.pack('>Q', int(time.time() * 1000)) + b'\x00\xff\xff\x00\xfe\xfe\xfe\xfe\xfd\xfd\xfd\xfd\x12\x34\x56\x78' + b'\x00'*8

print('Sending RakNet ping to', target)
sock.sendto(packet, target)

try:
    data, addr = sock.recvfrom(2048)
    print('Received response:', data)
    if len(data) > 35 and data[0] == 0x1c:
        print('MOTD:', data[35:].decode('utf-8', errors='ignore'))
except socket.timeout:
    print('Timeout on LAN IP')

# Also try 127.0.0.1
target_lo = ('127.0.0.1', 7551)
print('Sending RakNet ping to', target_lo)
sock.sendto(packet, target_lo)
try:
    data, addr = sock.recvfrom(2048)
    print('Received response:', data)
    if len(data) > 35 and data[0] == 0x1c:
        print('MOTD:', data[35:].decode('utf-8', errors='ignore'))
except socket.timeout:
    print('Timeout on Loopback')
