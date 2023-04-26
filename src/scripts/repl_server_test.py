import itertools
import random
import string

with open("./src/scripts/repl_server.py") as f:
    code = f.readlines()

code.insert(0, "__PORT__ = 9000\n")
code = itertools.takewhile(lambda l: not l.startswith("# DummyVM"), code)

exec("".join(code))


class MockSocket:
    def __init__(self):
        self.data = bytearray()
        self.cursor = 0

    def send(self, data):
        self.data.extend(data)

    def recv(self, bufsize):
        if self.cursor > len(self.data):
            raise Exception(f"MockSocket: recv({bufsize}) out of range")
        data = bytes(self.data[self.cursor : self.cursor + bufsize])
        self.cursor += bufsize
        return data

corr_data = "".join(random.choices(string.ascii_uppercase + string.digits, k=2048))
s = MessageStream(MockSocket())

s.send_msg(INST.PRINT, corr_data)
inst, recv_data = s.recv_msg()
assert inst == INST.PRINT
assert recv_data == corr_data

s.send_msg(INST.EXIT, "")
inst, recv_data = s.recv_msg()
assert inst == INST.EXIT
assert recv_data == ""
