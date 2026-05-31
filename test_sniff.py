from scapy.all import sniff, UDP

def pkt_callback(pkt):
    if pkt.haslayer(UDP):
        payload = bytes(pkt[UDP].payload)
        if b'MCPE' in payload:
            print(f"Found MCPE from {pkt[1].src}:{pkt[UDP].sport} -> {pkt[1].dst}:{pkt[UDP].dport}")
            print("Payload:", payload)
        elif len(payload) > 20 and payload[0] == 0x1c:
            print(f"Found RakNet Pong from {pkt[1].src}:{pkt[UDP].sport} -> {pkt[1].dst}:{pkt[UDP].dport}")
            print("Payload:", payload)

print("Sniffing UDP packets for MCPE broadcasts...")
sniff(filter="udp", prn=pkt_callback, store=0, timeout=15)
