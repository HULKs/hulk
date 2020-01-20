from datetime import datetime
from pathlib import Path
import re
import curses

import gevent
import gevent.subprocess as subprocess
from pssh.clients import ParallelSSHClient
from pssh.output import HostOutput


def naoToIP(id):
    """Attempts to convert the given identifier string to an IP
    If the id is a number, it is converted to 10.1.24.<id+10>
    If the id is a number followed by a "w", it is converted to 10.0.24.<id+10>
    IPs and any other strings (assuming hostname) are passed through unchanged
    """
    if len(id.split(".")) == 4:  # pass through raw ips
        return id
    s = str(id)
    isLAN = 1
    if s.endswith("w"):  # use subnet 1 for lan, 0 for wifi
        s = s[:-1]
        isLAN = 0

    if s.isdigit():
        return f"10.{isLAN}.24.{int(s)+10}"
    else:
        # assume id is a hostname
        return id


def isV5(number):
    return number < 20


def selectByVersion(numbers, v5command, v6command):
    """For every robot, the appropriate command is selected"""
    commands = [(v5command if isV5(number) else v6command)
                for number in numbers]
    return commands


class NaoSSH(ParallelSSHClient):

    """A client to send/receive files to/from one or multiple naos and run commands on them."""

    logpaths = [
        "/home/nao/naoqi/tuhhNao.*",
        "/home/nao/naoqi/filetransport_*",
        "/mnt/usb/filetransport_*",
        "/home/nao/naoqi/replay_*",
        "/mnt/usb/replay_*"
    ]

    def __init__(self, hosts, *args, **kwargs):
        """Arguments:
        hosts - hostnames
        pkey  - path to private key file (required for scp actions)
        """
        self.ips = [naoToIP(host) for host in hosts]
        self.numbers = [(int(ip.split(".")[-1])-10)
                        if ip.isdigit() else 99 for ip in self.ips]
        self.pkey = kwargs.get("pkey", "")
        super().__init__(self.ips, *args, **kwargs)

    def process_output(self, stdout, output_queue, output_filter=None, process=None, raise_error=False):
        """Helper function to read from a text stream asynchroneously

        Arguments:
        stdout             - stream to read from
        output_queue       - queue to append lines to
        output_filter      - regex to filter lines before appending to the queue
        process            - if raise_error, check return code, raising an exception if it's non-zero
        raise_error        - see above
        """
        lines = []
        for line in stdout:
            if type(line) is bytes:
                line = line.decode()
            lines.append(line)
            if output_filter:
                match = re.search(output_filter, line)
                if match:
                    output_queue.put(match.group(1))
                continue
            output_queue.put(line)

        if process and raise_error:
            exitcode = process.wait()
            if exitcode != 0:
                raise subprocess.CalledProcessError(exitcode, process, lines)

    def run_local_command(self, command, output_queue=None, output_filter=None, raise_error=False):
        """Run a shell command on this machine without blocking

        Arguments:
        command       - command to run
        output_queue  - passed to process_output if given
        output_filter - passed to process_output
        raise_error   - passed to process_output
        """
        process = subprocess.Popen(
            command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        if output_queue:
            return gevent.spawn(self.process_output, process.stdout, output_queue, output_filter=output_filter, process=process, raise_error=raise_error)
        return process

    def scp(self, source, destinations, output_queue=None, raise_error=False):
        """Copy files via scp

        Arguments:
        source       - source path
        destinations - destination path
        output_queue - passed to process_output
        raise_error  - passed to process_output
        """
        keyfile = ""
        if self.pkey:
            keyfile = " -i " + self.pkey

        jobs = []
        for host, destination in zip(self.hosts, destinations):
            command = f"scp -v -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o LogLevel=quiet{keyfile} -r nao@{host}:{source} '{destination}/'"
            jobs += [self.run_local_command(command, output_queue,
                                            r"(?<=Sink: C.... )(.*)", raise_error)]
        return jobs

    def setNetwork(self, network):
        """Run the set network script on the target"""
        return self.run_command("%s", host_args=selectByVersion(self.numbers,
                                                                v5command=f"/home/nao/bin/setNetwork {network}",
                                                                v6command=f"/data/home/nao/.local/bin/setNetwork {network}"))

    def hulk(self, parameters):
        """Interact with the hulk service on the target"""
        return self.run_command("%s", host_args=selectByVersion(self.numbers,
                                                                v5command=f"sudo /etc/init.d/hulk {parameters}",
                                                                v6command=f"systemctl --user {parameters} hulk.service"))

    def shutdown(self, reboot=False):
        """Shut down or reboot the target nao(s)"""
        if reboot:
            return self.run_command("%s", host_args=selectByVersion(self.numbers,
                                                                    v5command="sudo shutdown -r now",
                                                                    v6command="systemctl reboot"))
        else:
            return self.run_command("%s", host_args=selectByVersion(self.numbers,
                                                                    v5command="sudo shutdown -h now",
                                                                    v6command="systemctl poweroff"))

    def downloadLogs(self, logdir, output_queue=None, raise_error=False):
        """Download logs from the target"""
        def dmesg(dirs):
            result = self.run_command("dmesg")
            for i, host in enumerate(result.values()):
                filename = datetime.now().strftime("%Y-%m-%d_%H-%M-%S_dmesg.log")
                with open(dirs[i] / filename, "w") as logfile:
                    for line in host.stdout:
                        logfile.write(line+"\n")
                    for line in host.stderr:
                        output_queue.put(line)
                    output_queue.put("{} {}".format(logfile.tell(), filename))

        logdir = Path(logdir)
        destinations = [logdir / str(n) for n in self.ips]
        for d in destinations:
            d.mkdir(parents=True, exist_ok=True)
        tasks = []

        # check size/existence of logs
        result = self.run_command(
            "du -s --block-size=1 " + " ".join(self.logpaths))
        self.join(result)
        logpaths = []
        size_total = 0
        output_queue.put(
            (curses.A_BOLD, "Copying from the following locations:"))
        for line in list(result.values())[0].stdout:
            size, path = re.match(r"(\d+)\s+(\S*)", line).group(1, 2)
            size = int(size)
            if size > 0:
                logpaths.append(path)
                size_total += size

        # warn if there are no logs
        if len(logpaths) == 0:
            output_queue.put(
                (curses.color_pair(4) + curses.A_BOLD, "WARNING: No logs found!"))
        output_queue.put(("    " + "\n    ".join(logpaths) + "\n"))

        # download files
        for path in logpaths:
            tasks += self.scp(path, destinations, output_queue, raise_error)

        # download dmesg
        tasks.append(gevent.spawn(dmesg, destinations))
        return tasks, size_total

    def deleteLogs(self):
        return self.run_command("rm -vfr " + " ".join(self.logpaths))
