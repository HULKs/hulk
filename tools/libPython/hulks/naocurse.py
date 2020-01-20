import curses

import gevent


class NaoCurse:

    def __init__(self, h, w, title=""):
        self.title = title
        self.status = ""
        self.buffer = []
        self.scroll = 0
        self.animation = -1
        self.color = 0
        self.queue = gevent.queue.Queue()
        self.box = curses.newwin(h, w)

    def process_queue(self):
        """Empty queue and append the content to the buffer"""
        while not self.queue.empty():
            try:
                block = self.queue.get_nowait()
                modifier = 0
                if type(block) is tuple:
                    modifier, text = block
                else:
                    text = block
                for line in text.split("\n"):
                    if type(line) is str:
                        self.buffer.append((modifier, line))
            except gevent.queue.Empty as e:
                pass
            except Exception as e:
                self.buffer.append((curses.color_pair(2), str(type(block))))
                self.buffer.append((curses.color_pair(2), str(e)))

    def refresh(self):
        """Render ncurses window"""
        status = self.title + " - " + self.status
        if self.animation >= 0:
            self.animation += 1
            status += " " + "/-\\|"[self.animation % 4]
        self.process_queue()
        h, w = self.box.getmaxyx()
        self.box.clear()
        if h < 2:
            self.box.hline(0, 1, curses.ACS_HLINE, w-1)
        else:
            self.box.box()
        self.box.addstr(0, 2, self.title, curses.A_BOLD)
        self.box.addstr(0, w - 2 - len(self.title), self.title, curses.A_BOLD)
        self.box.addstr(0, w // 2 - len(status) // 2,
                        status, curses.A_BOLD + self.color)
        offset = max(0, len(self.buffer) - h + 2)
        for i in range(h-2):
            if i+offset+self.scroll in range(len(self.buffer)):
                modifier, line = self.buffer[i+offset+self.scroll][:w-2]
                self.box.addstr(i + 1, 1, line, modifier)

        self.box.refresh()

    def set_status(self, status, animation=None, color=None):
        self.status = status
        if animation is not None:
            self.animation = max(0, self.animation) if animation else -1
        if color is not None:
            self.color = color
