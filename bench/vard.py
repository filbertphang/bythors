import random
import socket
import re
import uuid
from select import select
from struct import pack, unpack
import time

def poll(sock, timeout):
    return sock in select([sock], [], [], timeout)[0]

class SendError(Exception):
    pass

class ReceiveError(Exception):
    pass

class LeaderChanged(Exception):
    pass

class Client(object):
    class NoLeader(Exception):
        pass

    @classmethod
    def find_leader(cls, cluster):
        # cluster should be a list of [(host, port)] pairs
        for (host, port) in cluster:
            try:
                print "probing node " + host + ":" + str(port)
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(1.0)
                sock.connect((host, port))
                sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                c = cls(host, port, sock)
                c.get('a')
            except socket.error as e:
                print "socket.error", e
                continue
            except socket.timeout as e:
                print "socket.timeout", e
                continue
            except LeaderChanged as e:
                print "probed node is not leader"
                # This happens if the server contacted is not the leader.
                # Since leader detection works by trying servers one by one, this is expected.
                continue
            else:
                print "leader is " + host + ":" + str(port)
                return (host, port)
        print "No leader found. Leader-election may not have terminated yet. Try again in a few seconds."
        raise cls.NoLeader

    response_re = re.compile(r'Response\W+([0-9]+)\W+([/A-Za-z0-9]+|-)\W+([/A-Za-z0-9]+|-)\W+([/A-Za-z0-9]+|-)')

    def __init__(self, host, port, sock=None, timeout=None, client_id=None, request_id=None):
        if client_id is None:
            client_id = random.randint(1, 2**31 - 1)

        self.client_id = client_id

        if not sock:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

            if timeout is None:
                timeout = 10.0

            self.sock.settimeout(timeout)
            self.sock.connect((host, port))
        else:
            self.sock = sock

        if request_id is None:
            request_id = 0

        self.request_id = request_id

    def deserialize(self, data):
        if data == '-':
            return None
        return data

    def serialize(self, arg):
        if arg is None:
            return '-'
        return str(arg)


    def send_command(self, cmd, arg1=None, arg2=None, arg3=None):
        msg = str(self.client_id) + ' ' + str(self.request_id) + ' ' + cmd + ' ' + ' '.join(map(self.serialize, (arg1, arg2, arg3)))
        self.sock.send(pack("<I", len(msg)) + msg)
        self.request_id += 1

    def parse_response(self, data):
        if data.startswith('NotLeader'):
            raise LeaderChanged
        try:
            match = self.response_re.match(data)
            return [self.deserialize(match.group(n)) for n in (1,2,3,4)]
        except Exception as e:
            print "Parse error, data=%s" % data
            raise e

    def process_response(self):
        len_bytes = self.sock.recv(4)
        if len_bytes == '':
            raise ReceiveError
        else:
            len_msg = unpack("<I", len_bytes)[0]
            data = self.sock.recv(len_msg)
            if data == '':
                raise ReceiveError
            else:
                return self.parse_response(data)

    def get(self, k):
        self.send_command('GET', k)
        return self.process_response()[2]

    def get_no_wait(self, k):
        self.send_command('GET', k)

    def put_no_wait(self, k, v):
        self.send_command('PUT', k, v)

    def put(self, k, v):
        self.send_command('PUT', k, v)
        return self.process_response()[2]

    def delete(self, k):
        self.send_command('DEL', k)
        self.process_response()[2]

    def compare_and_set(self, k, current, new):
        self.send_command('CAS', k, current, new)
        if self.process_response()[3] == new:
            return True
        return False

    def compare_and_delete(self, k, current):
        self.send_command('CAD', k, current)
        if self.process_response()[3] is None:
            return True
        return False

class FailoverTolerantClient(object):
    def __init__(self, cluster, timeout=2.0, retries=10, no_leader_sleep=1.0):
        self.cluster = cluster
        self.timeout = timeout
        self.retries = retries
        self.no_leader_sleep = no_leader_sleep

        self.client = None
        self.client = self.get_client_at_leader()

    def get_client_at_leader(self):
        for i in range(self.retries):
            try:
                (host, port) = Client.find_leader(self.cluster)
                if self.client is None:
                    client_id = None
                    request_id = None
                else:
                    client_id = self.client.client_id
                    request_id = self.client.request_id

                return Client(host, port, timeout=self.timeout, client_id=client_id, request_id=request_id)
            except Client.NoLeader:
                print "sleeping in hopes a leader gets elected"
                time.sleep(self.no_leader_sleep)
                continue
        print "too many retries, giving up"
        raise Client.NoLeader

    def wrap_client_method(self, method, *args):
        while True:
            try:
                return method(self.client, *args)
            except (socket.error, socket.timeout, SendError, ReceiveError, LeaderChanged) as e:
                print "Failover at %s due to %s" % (time.time(), e)
                self.client = self.get_client_at_leader()
                print "found new leader!"

    def get(self, k):
        return self.wrap_client_method(Client.get, k)

    def put(self, k, v):
        return self.wrap_client_method(Client.put, k, v)

    def delete(self, k):
        return self.wrap_client_method(Client.delete, k)

    def compare_and_set(self, k, current, new):
        return self.wrap_client_method(Client.compare_and_set, k, current, new)
