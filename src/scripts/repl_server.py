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

def __encode(s):
    s_bytes = s.encode()
    s_len = len(s_bytes)
    # two bytes for size, and n bytes for data
    return s_len.to_bytes(2, 'big') + s_bytes

while True:
    try:
        __order = __client_socket.recv(1024).decode()
    except ConnectionResetError: # when the server was crashed
        break
    if __order == 'quit' or __order == 'exit': # when the server was closed successfully
        __client_socket.send(__encode('closed'))
        break
    elif __order == 'load':
        __sys.stdout = __io.StringIO()
        __res = ''
        __exc = ''
        __buf = []
        try:
            if __already_loaded:
                # __MODULE__ will be replaced with module name
                __res = str(exec('__importlib.reload(__MODULE__)', __ctx))
            else:
                __res = str(exec('import __MODULE__', __ctx))
            __already_loaded = True
        except SystemExit:
            __buf.append('[Exception] SystemExit')
            continue
        except Exception as e:
            try:
                excs = __traceback.format_exception(e)
            except:
                excs = __traceback.format_exception_only(e.__class__, e)
            __exc = ''.join(excs).rstrip()
            __traceback.clear_frames(e.__traceback__)
            __buf.append('[Initialize]')
        __out = __sys.stdout.getvalue()[:-1]
        # assert not(__exc and __res)
        if __exc or __res:
            __out += '\n'
        __res = __out + __exc + __res
        __buf.append(__res)
        __client_socket.send(__encode(''.join(__buf)))
    else:
        __client_socket.send(__encode('unknown operation'))

__client_socket.close()
__server_socket.close()
