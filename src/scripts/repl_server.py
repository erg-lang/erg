# All strings must be quoted by single quotes to prevent shell interpretation
import socket
import sys
import importlib
import io
import traceback

class INST:
    # Informs that it is not a supported instruction.
    UNKNOWN = 0x00
    # Send from server to client. Informs the client to print data.
    PRINT = 0x01
    # Send from client to server. Informs the REPL server that the executable .pyc file has been written out and is ready for evaluation.
    LOAD = 0x02
    # Send from server to client. Represents an exception.
    EXCEPTION = 0x03
    # Send from server to client. Tells the code generator to initialize due to an error.
    INITIALIZE = 0x04
    # Informs that the connection is to be / should be terminated.
    EXIT = 0x05

class MessageStream:
    def __init__(self, socket):
        self.socket = socket
        self._read_buf = bytearray()

    def recv_msg(self):
        self._read_buf.clear()
        # requires at least 3 bytes as metadata
        while len(self._read_buf) < 3:
            self._read_buf.extend(self.socket.recv(1024))

        inst = int.from_bytes(self._read_buf[:1], 'big')
        data_len = int.from_bytes(self._read_buf[1:3], 'big')

        # until all data has been read
        while len(self._read_buf) < 3 + data_len:
            self._read_buf.extend(self.socket.recv(1024))

        return (inst, self._read_buf[3:].decode())


    def send_msg(self, inst, data=''):
        data_bytes = data.encode()
        data_len = len(data_bytes)
        # one byte for inst, two bytes for size, and n bytes for data(Optional)
        raw_bytes = inst.to_bytes(1, 'big') + data_len.to_bytes(2, 'big') + data_bytes

        self.socket.send(raw_bytes)

    def close():
        self.socket.close()

server_socket = socket.socket()
# DummyVM will replace this __PORT__ with free port
server_socket.bind(('127.0.0.1', __PORT__))
server_socket.listen(1)
(client_socket, client_address) = server_socket.accept()

already_loaded = False
ctx = {'importlib': importlib}
client_stream = MessageStream(client_socket)

while True:
    try:
        inst, _ = client_stream.recv_msg()
    except ConnectionResetError: # when the server was crashed
        break
    if inst == INST.EXIT: # when the server was closed successfully
        client_stream.send_msg(INST.EXIT)
        break
    elif inst == INST.LOAD:
        sys.stdout = io.StringIO()
        res = ''
        exc = ''
        resp_inst = INST.PRINT
        buf = []
        try:
            if already_loaded:
                # __MODULE__ will be replaced with module name
                res = str(exec('importlib.reload(__MODULE__)', ctx))
            else:
                res = str(exec('import __MODULE__', ctx))
            already_loaded = True
        except SystemExit:
            client_stream.send_msg(INST.EXCEPTION, 'SystemExit')
            continue
        except Exception as e:
            try:
                excs = traceback.format_exception(e)
            except:
                excs = traceback.format_exception_only(e.__class__, e)
            exc = ''.join(excs).rstrip()
            traceback.clear_frames(e.__traceback__)
            resp_inst = INST.INITIALIZE
        out = sys.stdout.getvalue()[:-1]
        if out and exc or res:
            out += '\n'
        res = out + exc + res
        buf.append(res)
        client_stream.send_msg(resp_inst, ''.join(buf))
    else:
        client_stream.send_msg(INST.UNKNOWN)

client_stream.close()
server_socket.close()
