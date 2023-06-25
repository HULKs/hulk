import numpy


residuals_per_iteration = []
parameters_per_iteration = []
with open("log.txt") as f:
    for line in f:
        line = line.strip()
        if line.startswith("residuals()"):
            assert line[39] == "[" and line[-35] == "]"
            line = line[40:-35]
            residuals = numpy.array([float(residual) for residual in line.split(", ")])
            residuals_per_iteration.append(float(numpy.sum(residuals**2)))
        elif line.startswith("set_params"):
            assert line[12] == "[" and line[-3] == "]"
            line = line[13:-3]
            parameters = [float(residual) for residual in line.split(", ")]
            parameters_per_iteration.append(parameters)


print(residuals_per_iteration)
for parameters in parameters_per_iteration:
    print(parameters)
