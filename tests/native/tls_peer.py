"""Isolated loopback TLS/SIP fixture. Keys stay in caller's ignored build directory."""
import base64, hashlib, os, pathlib, re, socket, ssl, subprocess, sys, threading
root=pathlib.Path(sys.argv[1]); mode=sys.argv[2]; root.mkdir(parents=True, exist_ok=True)
def run(*args):
    subprocess.run(['openssl',*args],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
if not (root/'server.pem').exists():
    run('req','-x509','-newkey','rsa:2048','-nodes','-days','1','-subj','/CN=Ruf Test Root','-addext','basicConstraints=critical,CA:TRUE','-keyout',str(root/'ca.key'),'-out',str(root/'ca.pem'))
    run('req','-newkey','rsa:2048','-nodes','-subj','/CN=localhost','-keyout',str(root/'server.key'),'-out',str(root/'server.csr'))
    (root/'extensions').write_text('subjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n')
    run('x509','-req','-in',str(root/'server.csr'),'-CA',str(root/'ca.pem'),'-CAkey',str(root/'ca.key'),'-CAcreateserial','-days','1','-extfile',str(root/'extensions'),'-out',str(root/'server.pem'))
context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);context.load_cert_chain(root/'server.pem',root/'server.key')
listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen();port=listener.getsockname()[1]
rtp=socket.socket(socket.AF_INET,socket.SOCK_DGRAM);rtp.bind(('127.0.0.1',0));rtpport=rtp.getsockname()[1]
key=base64.b64encode(os.urandom(30)).decode()
lock=threading.Lock()
def log(line):
    with lock:
        with (root/'packets.log').open('a') as f:f.write(line+'\n')
def header(message,name):
    return next(line.split(':',1)[1].strip() for line in message.split('\r\n') if line.lower().startswith(name.lower()+':'))
def serve(raw):
    try:
        sock=raw if mode=='tcp' else context.wrap_socket(raw,server_side=True);log('CONNECTED' if mode=='tcp' else 'HANDSHAKE');pending=b''; incoming_sent=False
        while True:
            chunk=sock.recv(65536)
            if not chunk:return
            pending+=chunk
            while b'\r\n\r\n' in pending:
                head,body=pending.split(b'\r\n\r\n',1);text=head.decode();length=int(header(text,'Content-Length'))
                if len(body)<length:break
                message=(head+b'\r\n\r\n'+body[:length]).decode();pending=body[length:]
                method=message.split(' ',1)[0];log(message.split('\r\n')[0])
                if method=='REGISTER' and mode=='digest' and 'Authorization:' not in message:
                    sock.sendall((f'SIP/2.0 401 Unauthorized\r\nVia: {header(message,"Via")}\r\nFrom: {header(message,"From")}\r\nTo: {header(message,"To")};tag=auth\r\nCall-ID: {header(message,"Call-ID")}\r\nCSeq: {header(message,"CSeq")}\r\nWWW-Authenticate: Digest realm="fixture", nonce="synthetic-nonce", algorithm=MD5\r\nContent-Length: 0\r\n\r\n').encode());continue
                if method=='REGISTER' and mode=='digest':
                    auth=dict(re.findall(r'(\w+)="([^"]*)"',header(message,'Authorization')))
                    md5=lambda text:hashlib.md5(text.encode()).hexdigest()
                    expected=md5(md5('digest-user:fixture:synthetic-only')+':synthetic-nonce:'+md5('REGISTER:'+auth.get('uri','')))
                    if auth.get('username')!='digest-user' or auth.get('response')!=expected:raise ConnectionError('unexpected synthetic digest')
                    log('AUTH digest-user')
                if method=='SIP/2.0' and mode=='incoming' and message.startswith('SIP/2.0 200'):
                    target=header(message,'Contact').split('<')[1].split('>')[0]
                    ack=f'ACK {target} SIP/2.0\r\nVia: SIP/2.0/TLS 127.0.0.1:{port};branch=z9hG4bK-incoming-ack\r\nFrom: {header(message,"From")}\r\nTo: {header(message,"To")}\r\nCall-ID: {header(message,"Call-ID")}\r\nCSeq: 1 ACK\r\nContent-Length: 0\r\n\r\n'
                    sock.sendall(ack.encode());continue
                if method=='ACK':continue
                if method not in ('REGISTER','INVITE','BYE','INFO','REFER','OPTIONS'):continue
                to=header(message,'To');to=to if ';tag=' in to else to+';tag=tls-peer'
                contact=header(message,'Contact') if method=='REGISTER' else f'<sip:peer@127.0.0.1:{port}'+ (';transport=udp' if mode=='contactudp' else '')+'>'
                sdp=''
                if method=='INVITE':
                    proto='RTP/AVP' if mode in ('plaintext','tcp') else 'RTP/SAVP'
                    sdp=f'v=0\r\no=fixture 1 1 IN IP4 127.0.0.1\r\ns=fixture\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio {rtpport} {proto} 0\r\na=rtpmap:0 PCMU/8000\r\na=sendrecv\r\n'
                    if mode not in ('missingcrypto','plaintext','tcp'):sdp+=f'a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:{"AAAA" if mode=="badkey" else key}\r\n'
                extra='Content-Type: application/sdp\r\n' if sdp else ''
                response=f'SIP/2.0 200 OK\r\nVia: {header(message,"Via")}\r\nFrom: {header(message,"From")}\r\nTo: {to}\r\nCall-ID: {header(message,"Call-ID")}\r\nCSeq: {header(message,"CSeq")}\r\nContact: {contact}\r\nExpires: 60\r\n{extra}Content-Length: {len(sdp)}\r\n\r\n{sdp}'
                sock.sendall(response.encode())
                if method=='REGISTER' and mode=='incoming' and not incoming_sent:
                    incoming_sent=True
                    target=contact.split('<')[1].split('>')[0]
                    offer=f'v=0\r\no=fixture 1 1 IN IP4 127.0.0.1\r\ns=fixture\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio {rtpport} RTP/SAVP 0\r\na=rtpmap:0 PCMU/8000\r\na=sendrecv\r\na=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:{key}\r\n'
                    invite=f'INVITE {target} SIP/2.0\r\nVia: SIP/2.0/TLS 127.0.0.1:{port};branch=z9hG4bK-incoming\r\nFrom: <sip:caller@localhost>;tag=incoming-peer\r\nTo: <sip:tls-fixture@localhost>\r\nCall-ID: incoming-tls-test\r\nCSeq: 1 INVITE\r\nContact: <sip:peer@127.0.0.1:{port}>\r\nMax-Forwards: 70\r\nContent-Type: application/sdp\r\nContent-Length: {len(offer)}\r\n\r\n{offer}'
                    sock.sendall(invite.encode())
    except (ssl.SSLError,ConnectionError,OSError):pass
# A receiver on the identical UDP endpoint proves no signaling downgrade occurs.
udp=socket.socket(socket.AF_INET,socket.SOCK_DGRAM);udp.bind(('127.0.0.1',port))
def plaintext_receiver():
    while True:
        packet,_=udp.recvfrom(65535)
        if packet:log('PLAINTEXT RECEIVED')
threading.Thread(target=plaintext_receiver,daemon=True).start()
print(port,flush=True)
while True:
    raw,_=listener.accept();threading.Thread(target=serve,args=(raw,),daemon=True).start()
