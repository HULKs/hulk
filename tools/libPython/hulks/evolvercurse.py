import curses
import time
import datetime


class Pad(object):
    def __init__(self,
                 x = 10,
                 y = 10,
                 from_line = 0,
                 from_col = 60,
                 to_line = 20,
                 to_col = 120,
                 stdscr = None,
                 use_curses = True):
        if use_curses:
            self.pad = curses.newpad(x, y)
        self.stdscr = stdscr
        self.line = 0
        self.from_line = from_line
        self.from_col = from_col
        self.to_line = to_line
        self.to_col = to_col
        self.x = x
        self.y = y
        self.use_curses = use_curses

    def put_line(self, text):
        if self.use_curses:
            try:
                self.pad.addstr(self.line, 0, text[:self.y])
            except Exception as e:
                print("Whats wrong ?", e)
            self.line += 1
            par_height, par_width = self.stdscr.getmaxyx()
            
            try:
                self.pad.refresh(0,0, 
                                min(self.from_line, par_height-1),
                                min(self.from_col,par_width-1),
                                min(self.to_line, par_height-1),
                                min(self.to_col, par_width)-1)
            except Exception as e:
                print("Window too small ?")
        else:
            print(text)

    def print(self, members):
        if self.use_curses:
            self.pad.clear()


class PopulationPad(Pad): 
    def __init__(self,
                 x = 26,
                 y = 61,
                 from_line = 3,
                 from_col = 59,
                 to_line = 29,
                 to_col = 120,
                 stdscr = None,
                 use_curses = True):
        super(PopulationPad, self).__init__(x,
                                            y,
                                            from_line,
                                            from_col,
                                            to_line,
                                            to_col,
                                            stdscr,
                                            use_curses)

    def print(self, population):
        if self.use_curses:
            self.pad.clear()
        self.line = 0
        self.put_line("Population: (#pop: " + str(len(population.members)) +
                      ", " + str(population.generation) + " generations)")
        self.put_line("name               fitness   cost      score")
        for ind in population.members:
            self.put_line(ind.name[:18].ljust(18, ' ') +
                          " " + str(ind.fitness)[:6].ljust(6, ' ') +
                          "    " + str(ind.cost)[:6].ljust(6, ' ')+
                          "    " + str(ind.score)[:6].ljust(6, ' '))


class StatusPad(Pad):
    def __init__(self,
                 x = 10,
                 y = 58,
                 from_line = 3,
                 from_col = 0,
                 to_line = 13,
                 to_col = 58,
                 stdscr = None,
                 use_curses = True):
        super(StatusPad, self).__init__(x,
                                        y,
                                        from_line,
                                        from_col,
                                        to_line,
                                        to_col,
                                        stdscr,
                                        use_curses)
        self.line_buffer = []
        
    def print(self, text):
        ts = time.time()
        timestamp = datetime.datetime.fromtimestamp(ts).strftime('%H:%M:%S')
        self.line_buffer = self.line_buffer + ["[" + timestamp + "] " + text]
        self.line_buffer = self.line_buffer[-9:]
        if self.use_curses:
            self.pad.clear()
        self.line = 0
        if self.use_curses:
            self.put_line("Status:")
            for line in self.line_buffer:
                self.put_line(line)
        else:
            self.put_line("\nStatus: [" + timestamp + "] " + text + "\n")


class MemberPad(Pad):
    def __init__(self,
                 x = 35,
                 y = 47,
                 from_line = 14,
                 from_col = 0,
                 to_line = 49,
                 to_col = 47,
                 stdscr = None,
                 use_curses = True):
        super(MemberPad, self).__init__(x,
                                        y,
                                        from_line,
                                        from_col,
                                        to_line,
                                        to_col,
                                        stdscr,
                                        use_curses)
        
    def print(self, member, nn_type, final_layer_neurons):
        if self.use_curses:
            self.pad.clear()
        self.line = 0
        self.put_line("Individual: " + member.name)
        self.put_line("Type: " + nn_type)
        self.put_line("Training Epochs: " + str(member.genes["trainingEpochs"]))
        self.put_line("Optimizer: " + member.genes["optimizer"])
        self.put_line("Initial learning rate: " + str(member.genes["initial_learning_rate"]))
        self.put_line("Learning rate factor per epoch: " + str(member.genes["learning_rate_factor_per_epoch"]))
        self.put_line("Cost: " + str(member.cost))
        self.put_line("")
        self.put_line("Convolutional layers:")
        self.put_line("")
        self.put_line("kernels   batchnorm          pool")
        self.put_line("   size&stride   activation        dropout")
        output_size = 32
        outputs = output_size*output_size*1
        for each in member.genes["conv_layers"]:
            (t, n, k ,act, x, bn, dr, s) = each
            d = "  "
            if t == "SeparableConv2D":
                d= "S:"
            ex = x
            pooling = "max" + " " +str(ex) + "x" + str(ex)
            if x < 0:
                ex = -x
                pooling = "avg" + " " +str(ex) + "x" + str(ex)
            elif x == 0:
                pooling = "none"
            self.put_line((d + str(n).rjust(3," ")).ljust(6, " ") +
                            str(k) + "x" + str(k) + " " + str(s) +
                            " " + str(bn).ljust(7," ") +
                            "" + act[:6].ljust(7," ") +
                            pooling.ljust(10," ") + str(dr)[:4])
            output_size /= s
            if x != 0:
                output_size /= abs(x)
            output_size = int(output_size)
            outputs = output_size*output_size*n
        self.put_line("")
        self.put_line("Flatten: " + str(int(outputs)))
        self.put_line("")
        self.put_line("Dense Layers:")
        self.put_line("")
        self.put_line("neurons batchnorm activation dropout")
        for each in member.genes["dense_layers"]:
            (n, act, bn, dr) = each
            self.put_line("" + str(n).rjust(5," ") +
                          "   " + str(bn).ljust(8," ") +
                          "  " + act.ljust(13," ") + str(dr)[:4])
        (act,bn) = member.genes["final_layer"]
        self.put_line("" + str(final_layer_neurons).rjust(5," ") +
                      "   " + str(bn).ljust(8," ") +
                      "  " + act.ljust(11," "))


class SpamPad(Pad):
    def __init__(self,
                 x = 2,
                 y = 120,
                 from_line = 0,
                 from_col = 0,
                 to_line = 2,
                 to_col = 120,
                 stdscr = None,
                 use_curses = True):
        super(SpamPad, self).__init__(x,
                                      y,
                                      from_line,
                                      from_col,
                                      to_line,
                                      to_col,
                                      stdscr,
                                      use_curses)

    def print(self, epoch = ""):
        if self.use_curses:
            self.pad.clear()
        self.line = 0
        self.put_line(epoch)


class ProgressionPad(Pad):
    def __init__(self,
                 x = 30,
                 y = 72,
                 from_line = 28,
                 from_col = 48,
                 to_line = 58,
                 to_col = 120,
                 stdscr = None,
                 use_curses = True):
        super(ProgressionPad, self).__init__(x,
                                             y,
                                             from_line,
                                             from_col,
                                             to_line,
                                             to_col,
                                             stdscr,
                                             use_curses)

    def print(self, progression):
        if self.use_curses:
            self.pad.clear()
        self.line = 0
        self.put_line("Progression:")
        self.put_line("  gen #pop name               " + 
                      "fitness   cost      score")
        for (gen, score, fitness, cost, name, members, genes) in progression:
            self.put_line(str(gen).rjust(4, ' ') + " " +
                          str(members).rjust(4, ' ') + "  " +
                          str(name)[:18].ljust(18, ' ') + " " +
                          str(fitness)[:6].ljust(6, ' ') + "    " +
                          str(cost)[:6].ljust(6, ' ') + "    " +
                          str(score)[:6].ljust(6, ' ') + "     ")
