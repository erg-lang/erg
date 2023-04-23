# Append __ to all variables to prevent name collisions in exec
# All strings must be quoted by single quotes to prevent shell interpretation
import socket as __socket
import sys as __sys
import importlib as __importlib
import io as __io
import traceback as __traceback

__server_socket = __socket.socket()
# DummyVM will replace this __PORT__ with free port
__server_socket.bind(('127.0.0.1', __PORT__))
__server_socket.listen(1)
(__client_socket, __client_address) = __server_socket.accept()

__already_loaded = False
__ctx = {'__importlib': __importlib}


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

def __encode(instr, data=''):
    data_bytes = data.encode()
    data_len = len(data_bytes)
    if data_len > 0:
        # one byte for inst, two bytes for size(Optional), and n bytes for data(Optional)
        return instr.to_bytes(1, 'big') + data_len.to_bytes(2, 'big') + data_bytes
    return instr.to_bytes(1, 'big')


while True:
    try:
        __data = __client_socket.recv(1024)
    except ConnectionResetError: # when the server was crashed
        break
    __inst = int.from_bytes(__data[:1], 'big')
    if __inst == INST.EXIT: # when the server was closed successfully
        __client_socket.send(__encode(INST.EXIT))
        break
    elif __inst == INST.LOAD:
        __sys.stdout = __io.StringIO()
        __res = ''
        __exc = ''
        __resp_inst = INST.PRINT
        __buf = []
        try:
            if __already_loaded:
                # __MODULE__ will be replaced with module name
                __res = str(exec('__importlib.reload(__MODULE__)', __ctx))
            else:
                __res = str(exec('import __MODULE__', __ctx))
            __already_loaded = True
        except SystemExit:
            __client_socket.send(__encode(INST.EXCEPTION, 'SystemExit'))
            continue
        except Exception as e:
            try:
                excs = __traceback.format_exception(e)
            except:
                excs = __traceback.format_exception_only(e.__class__, e)
            __exc = ''.join(excs).rstrip()
            __traceback.clear_frames(e.__traceback__)
            __resp_inst = INST.INITIALIZE
        __out = __sys.stdout.getvalue()[:-1]
        if __out and __exc or __res:
            __out += '\n'
        __res = __out + __exc + __res
        __buf.append(__res)
        __client_socket.send(__encode(__resp_inst, ''.join(__buf)))
    else:
        __client_socket.send(__encode(INST.UNKNOWN))

__client_socket.close()
__server_socket.close()
