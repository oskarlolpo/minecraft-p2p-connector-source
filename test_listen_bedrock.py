import socket

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind(('0.0.0.0', 19132))
sock.settimeout(10.0)

print('Listening on 0.0.0.0:19132 for 10 seconds...')
try:
    while True:
        data, addr = sock.recvfrom(2048)
        print(f'Received {len(data)} bytes from {addr}:')
        if b'MCPE' in data:
            print('Found MCPE string:', data)
        else:
            print(data.hex())
except socket.timeout:
    print('Timeout')
except Exception as e:
    print('Error:', e)
