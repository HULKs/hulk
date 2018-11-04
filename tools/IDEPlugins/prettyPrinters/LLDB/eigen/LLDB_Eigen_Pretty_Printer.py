import lldb
import re
import os
from functools import partial

def __lldb_init_module (debugger, dict):
    debugger.HandleCommand("type summary add -x \"^Eigen::Matrix<.*?>$\" -F\
                           LLDB_Eigen_Pretty_Printer.eigen_matrix_print -p -r\
                          -w Eigen")
    debugger.HandleCommand("type summary add -x \"^Eigen::Array<.*?>$\" -F\
                           LLDB_Eigen_Pretty_Printer.eigen_array_print -p -r\
                          -w Eigen")
    debugger.HandleCommand("type summary add -x \"^Eigen::Quaternion<.*?>$\" \
                           -F LLDB_Eigen_Pretty_Printer.eigen_quaternion_print\
                           -p -r -w Eigen")
    debugger.HandleCommand("type summary add -x \"^Eigen::SparseMatrix<.*?>$\"\
                           -F\
                           LLDB_Eigen_Pretty_Printer.eigen_sparsematrix_print\
                           -p -r -w Eigen")
    debugger.HandleCommand("type category enable Eigen")

def evaluate_expression(valobj, expr):
    return valobj.GetProcess().GetSelectedThread().GetSelectedFrame().EvaluateExpression(expr)

def evaluate_at_index(valobj, index):
    return valobj.GetValueForExpressionPath("["+str(index)+"]")

class Printer:
    def __init__(self, data):
        self.data = data

    def evaluate_real(self, index):
        return "%1.5e" % float(evaluate_at_index(self.data, index).GetValue())
                # float(self.data.GetValueForExpressionPath("["+str(index)+"]").GetValue())

    def evaluate_complex_double(self, index):
        val = \
        list(self.data.GetValueForExpressionPath("["+str(index)+"]._M_value").GetValue())
        val[-1] = 'j'
        for n in range(0, len(val)):
            if (val[n] == " "):
                del val[n]
                del val[n+1]
                if val[n+1] == "-":
                    del val[n]
                break
        val = complex("".join(val))

        return '{0:1.5e} {1} {2:1.5e}i'.format(val.real,\
                                               '+-'[val.imag < 0],\
                                               abs(val.imag))

    def evaluate_complex_int(self, index):
        # val = self.data.GetValueForExpressionPath("["+str(index)+"]")
        val = evaluate_at_index(self.data, index)
        real = val.GetValueForExpressionPath("._M_real").GetValueAsSigned()
        imag = val.GetValueForExpressionPath("._M_imag").GetValueAsSigned()
        val = real + imag * 1j

        return '{0:1.5e} {1} {2:1.5e}i'.format(val.real,\
                                               '+-'[val.imag < 0],\
                                               abs(val.imag))

class Matrix(Printer):
    def __init__(self, variety, val):
        try:
            valtype = val.GetType()
            type_str = valtype.GetDirectBaseClassAtIndex(0).GetName()

            begin = "Eigen::"+variety+"<"
            complex_scalar = "std::complex<"

            if (type_str.find(complex_scalar)>=0):
                regex = re.compile(begin + complex_scalar + ".*?>,.*?>")
                is_complex = True
            else:
                regex = re.compile(begin+".*?>")
                is_complex = False

            self.variety = regex.findall(type_str)[0]
            m = self.variety[len(begin):-1]
            template_params = m.split(",")
            # template_params = [x.replace(">", "") for x in template_params]
            template_params = [x.replace(" ", "") for x in template_params]

            self.rows = int(template_params[1])
            if self.rows == -1:
                self.rows = val.GetValueForExpressionPath(".m_storage.m_rows").GetValueAsSigned()

            self.cols = int(template_params[2])
            if self.cols == -1:
                self.cols = val.GetValueForExpressionPath(".m_storage.m_cols").GetValueAsSigned()

            self.options = 0
            if len(template_params) > 3:
                self.options = int(template_params[3])

            self.rowMajor = (int(self.options) & 0x1)
            self.innerType = template_params[0]

            if int(template_params[1]) == -1 or int(template_params[2]) == -1:
                data = val.GetValueForExpressionPath(".m_storage.m_data")
            else:
                data = val.GetValueForExpressionPath(".m_storage.m_data.array")

            Printer.__init__(self, data)
            if is_complex:
                val = self.data.GetValueForExpressionPath("[0]")
                if val.GetValueForExpressionPath("._M_value").IsValid():
                    self.get = partial(Printer.evaluate_complex_double, self)
                elif val.GetValueForExpressionPath("._M_real").IsValid():
                    self.get = partial(Printer.evaluate_complex_int, self)
                else:
                    self.variety = -1
            else:
                self.get = partial(Printer.evaluate_real, self)

        except:
            self.variety = -1

    def to_string(self, plain_print=False):
        padding = 1
        for i in range(0, self.rows * self.cols):
            padding = max(padding, len(str(self.get(i))))
        output = "["
        if not plain_print:
            output = "rows: %d, cols: %d - [" % (self.rows, self.cols)

        if (self.options):
            for i in range(0, self.rows):
                if i!=0:
                    output += " "

                for j in range(0, self.cols):
                    val = self.get(i*self.cols+j)
                    if j!=0:
                        output += val.rjust(padding+2, ' ')
                    else:
                        output += val.rjust(padding+1, ' ')

                if i!=self.rows-1:
                    output += ";"
        else:
            for i in range(0, self.rows):
                if i!=0:
                    output += " "

                for j in range(0, self.cols):
                    val = self.get(i+j*self.rows)
                    if j!=0:
                        output += val.rjust(padding+2, ' ')
                    else:
                        output += val.rjust(padding+1, ' ')

                if i!=self.rows-1:
                    output += ";"

        output+=" ]"

        return output

class SparseMatrix(Printer):
    def __init__(self, val):
        try:
            valtype = val.GetType()
            type_str = valtype.GetDirectBaseClassAtIndex(0).GetName()

            begin = "Eigen::SparseMatrix<"
            complex_scalar = "std::complex<"

            if (type_str.find(complex_scalar)>=0):
                regex = re.compile(begin + complex_scalar + ".*?>,.*?>")
                is_complex = True
            else:
                regex = re.compile(begin+".*?>")
                is_complex = False

            self.variety = regex.findall(type_str)[0]

            m = self.variety[len(begin):-1]
            template_params = m.split(",")
            template_params = [x.replace(" ", "") for x in template_params]

            self.options = 0

            if len(template_params) > 1:
                self.options = int(template_params[1])

            self.rowMajor = (int(self.options) & 0x1)
            self.innerType = template_params[0]

            if self.rowMajor:
                self.rows = \
                val.GetValueForExpressionPath(".m_outerSize").GetValueAsSigned()
                self.cols = \
                val.GetValueForExpressionPath(".m_innerSize").GetValueAsSigned()
            else:
                self.rows = \
                val.GetValueForExpressionPath(".m_innerSize").GetValueAsSigned()
                self.cols = \
                val.GetValueForExpressionPath(".m_outerSize").GetValueAsSigned()

            self.outerStarts = val.GetValueForExpressionPath(".m_outerIndex")
            self.innerNNZs = val.GetValueForExpressionPath(".m_innerNonZeros")
            self.size = \
            val.GetValueForExpressionPath(".m_data.m_size").GetValueAsSigned()
            self.indices = val.GetValueForExpressionPath(".m_data.m_indices")

            data = val.GetValueForExpressionPath(".m_data.m_values")
            Printer.__init__(self, data)

            if is_complex:
                val = self.data.GetValueForExpressionPath("[0]")
                if val.GetValueForExpressionPath("._M_value").IsValid():
                    self.get = partial(Printer.evaluate_complex_double, self)
                elif val.GetValueForExpressionPath("._M_real").IsValid():
                    self.get = partial(Printer.evaluate_complex_int, self)
                else:
                    self.variety = -1
            else:
                self.get = partial(Printer.evaluate_real, self)
        except:
            self.variety = -1

    def to_string(self):
        padding = 1

        for i in range(0, self.size):
            padding = max(padding, len(str(self.get(i))))

        output = "rows: %d, cols: %d - { " % (self.rows, self.cols)

        if (self.rowMajor):
            for i in range(0, self.rows):
                index = evaluate_at_index(self.outerStarts, \
                                          i).GetValueAsSigned()
                size = evaluate_at_index(self.innerNNZs, \
                                         i).GetValueAsSigned()

                for count in range(0, size):
                    j = evaluate_at_index(self.indices, \
                                          index+count).GetValueAsSigned()
                    output += "[%d, %d] =" % (i, j)
                    output += self.get(index+count).rjust(padding+1, ' ') + ", "
        else:
            rows = list()
            cols = list()
            vals = list()
            for j in range(0, self.cols):
                index = evaluate_at_index(self.outerStarts, \
                                          j).GetValueAsSigned()
                size = evaluate_at_index(self.innerNNZs, \
                                         j).GetValueAsSigned()

                for count in range(0, size):
                    i = evaluate_at_index(self.indices, \
                                          index+count).GetValueAsSigned()

                    val = self.get(index+count)
                    rows.append(i)
                    cols.append(j)
                    vals.append(val)

            indices = sorted(range(len(rows)), key=lambda k: rows[k])

            for index in indices:
                output += "[%d, %d] =" % (rows[index], cols[index])
                output += vals[index].rjust(padding+1, ' ') + ", "

        output += "\b\b }"

        return output


class Quaternion(Printer):
    def __init__(self, val):
        try:
            valtype = val.GetType()
            type_str = valtype.GetDirectBaseClassAtIndex(0).GetName()
            begin = "Eigen::Quaternion<"
            regex = re.compile(begin + ".*?>")

            self.variety = regex.findall(type_str)[0]

            m = self.variety[len(begin):-1]
            template_params = m.split(",")
            template_params = [x.replace(" ", "") for x in template_params]

            self.options = 0

            if len(template_params) > 1:
                self.options = int(template_params[1])

            self.DontAlign = (int(self.options) & 0x2)
            self.innerType = template_params[0]

            data = val.GetValueForExpressionPath(".m_coeffs.m_storage.m_data.array")
            Printer.__init__(self, data)
            self.variety = "Quaternion"
            self.get = partial(Printer.evaluate_real, self)
        except:
            self.variety = -1

    def to_string(self):
        padding = 1
        for i in range(0, 4):
            padding = max(padding, len(str(self.get(i))))

        output ="{ [x] = " + self.get(0) + ", " + \
                  "[y] = " + self.get(1) + ", " + \
                  "[z] = " + self.get(2) + ", " + \
                  "[w] = " + self.get(3) + " }"

        return output


def eigen_matrix_print (valobj, internal_dict):
    return eigen_matrix_print_plain_option(valobj, internal_dict, False)

def eigen_matrix_print_plain_option (valobj, internal_dict, plain_print):
    matrix = Matrix("Matrix", valobj)

    if (matrix.variety == -1):
        return "[EigenPrinter]: unable to print (Type: " + valobj.GetType().GetName() + " expected: Eigen::Matrix)"

    else:
        return matrix.to_string(plain_print)

def eigen_array_print (valobj,internal_dict):

    array = Matrix("Array", valobj)

    if (array.variety == -1):
        return "[EigenPrinter]: unable to print (Type: " + valobj.GetType().GetName() + " expected: Eigen::Array)"
    else:
        return array.to_string()

def eigen_quaternion_print (valobj,internal_dict):

    quaternion = Quaternion(valobj)

    if (quaternion.variety == -1):
        return "[EigenPrinter]: unable to print (Type: " + valobj.GetType().GetName() + " expected: Eigen::Quaternion)"
    else:
        return quaternion.to_string()

def eigen_sparsematrix_print (valobj,internal_dict):

    sparsematrix = SparseMatrix(valobj)

    if (sparsematrix.variety == -1):
        return "[EigenPrinter]: unable to print (Type: " + valobj.GetType().GetName() + " expected: Eigen::SparseMatrix)"
    else:
        return sparsematrix.to_string()
