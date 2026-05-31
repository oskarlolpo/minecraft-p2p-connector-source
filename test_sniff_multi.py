import socket
import threading

def listen_port(port):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    try:
        sock.bind(('', port))
        sock.settimeout(15.0)
        while True:
            data, addr = sock.recvfrom(2048)
            print(f'Port {port} recv from {addr}:')
            if b'MCPE' in data:
                print('Found MCPE:', data)
            else:
                print(data.hex())
    except socket.timeout:
        pass
    except Exception as e:
        print(f'Port {port} error: {e}')

t1 = threading.Thread(target=listen_port, args=(19132,))
t2 = threading.Thread(target=listen_port, args=(19133,))
t3 = threading.Thread(target=listen_port, args=(7551,))

print('Listening on 19132, 19133, 7551...')
t1.start()
t2.start()
t3.start()

t1.join()
t2.join()
t3.join()
