import socket
import select
import sys

def sniff():
    ports = [19132, 19133, 7551]
    sockets = []
    
    for p in ports:
        try:
            s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            # Some platforms need SO_BROADCAST
            s.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
            s.bind(('0.0.0.0', p))
            sockets.append((p, s))
            print(f"Listening for UDP broadcasts on port {p}...")
        except Exception as e:
            print(f"Failed to bind port {p}: {e}")

    if not sockets:
        print("No ports bound. Exiting.")
        return

    print("Waiting for packets (timeout 15s)...")
    end_time = time.time() + 15
    import time
    while time.time() < end_time:
        socks = [s for _, s in sockets]
        readable, _, _ = select.select(socks, [], [], 1.0)
        for s in readable:
            port = next(p for p, sock in sockets if sock == s)
            data, addr = s.recvfrom(2048)
            print(f"Packet from {addr} on port {port} (len={len(data)}): {data.hex()}")
            if b'MCPE' in data or len(data) > 30:
                try:
                    print("Decoded snippet:", data[33:].decode('utf-8', errors='ignore'))
                except:
                    pass

if __name__ == '__main__':
    sniff()
