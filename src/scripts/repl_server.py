# Append __ to all variables to prevent name collisions in exec
# All strings must be quoted by single quotes to prevent shell interpretation
import socket as __socket
import sys as __sys
import importlib as __importlib
import io as __io

__server_socket = __socket.socket()
# DummyVM will replace this __PORT__ with free port
__server_socket.bind(('127.0.0.1', __PORT__)) 
__server_socket.listen(1)
(__client_socket, __client_address) = __server_socket.accept()

__already_loaded = False
__res = ''

while True:
    __order = __client_socket.recv(1024).decode()
    if __order == 'quit' or __order == 'exit':
        __client_socket.send('closed'.encode())
        break
    elif __order == 'load':
        __sys.stdout = __io.StringIO()
        try:
            if __already_loaded:
                __res = str(exec('__importlib.reload(o)'))
            else:
                __res = str(exec('import o'))
        except SystemExit:
            __client_socket.send('[Exception] SystemExit'.encode())
            continue
        except e:
            __res = str(e)
        __already_loaded = True
        __out = __sys.stdout.getvalue().strip()
        __res = __out + '\n' + __res
        __client_socket.send(__res.encode())
    else:
        __client_socket.send('unknown operation'.encode())

__client_socket.close()
__server_socket.close()
