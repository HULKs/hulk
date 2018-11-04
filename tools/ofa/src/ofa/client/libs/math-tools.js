var math = require('mathjs');
var SVD = require('js-aruco').SVD;
/**
 * Plugging in "Psuedo inverse - pinv()" function for Math.JS library. This is
 * NOT perfect :P
 *
 * Uses SVD Library attached somewhere else.
 * TODO Bring in SVD code to this file.
 * Based on work of Nathan Zimmerman (https://github.com/moradology) and
 * other's.
 *
 */

math.pinv = function (A) {

    var prec = 2.220446049250313e-16;
    var tolerance = 1.e-64 / prec;
    A = math.matrix(A);
    /*AtransposeArr = math.transpose(A).toArray();
    let singular = true;

    for (let arr of AtransposeArr) {
        let sum = 0;
        for (let elem of arr) {
            elem = Math.abs(elem);
            if (elem < tolerance) {
                continue;
            }
            sum += elem;
        }
        if (sum === 0 || sum < tolerance) {
            // singular = false;
            break;
        }
    }
    if (!singular) {
        // CalibrationToolkit.printMat(A,"A");

        return math.inv(A);
    } else {

    }*/
    var u = math.matrix(A).toArray();
    var m = u.length;     // rows
    var n = u[0].length;  // cols

    var itmax = 50;
    var c = 0;
    var i = 0;
    var j = 0;
    var k = 0;
    var l = 0;

    // var u= numeric.clone(A);

    if (m != n) throw "non sq. matrix";
    if (m < n) {
        throw "Need more rows than columns"
    }
    // var e = new Array(n);
    var q = new Array(n);
    for (i = 0; i < n; i++) {
        /* e[i] =*/
        q[i] = 0.0;
    }
    // var v = rep([n, n], 0);
    //	v.zero();

    var v = math.zeros(n, n).toArray();
    var out = SVD.svdcmp(u, m, n, q, v);
    // vt= transpose(v)
    // return (u,q,vt)
    for (i = 0; i < q.length; i++)
        if (q[i] < prec)
            q[i] = 0

    // sort eigenvalues
    for (i = 0; i < n; i++) {
        // writeln(q)
        for (j = i - 1; j >= 0; j--) {
            if (q[j] < q[i]) {
                //  writeln(i,'-',j)
                c = q[j];
                q[j] = q[i];
                q[i] = c;
                for (k = 0; k < u.length; k++) {
                    temp = u[k][i];
                    u[k][i] = u[k][j];
                    u[k][j] = temp;
                }
                for (k = 0; k < v.length; k++) {
                    temp = v[k][i];
                    v[k][i] = v[k][j];
                    v[k][j] = temp;
                }
                //	   u.swapCols(i,j)
                //	   v.swapCols(i,j)
                i = j
            }
        }
    }

    var z = {U: u, S: q, V: v};
    var foo = z.S[0];
    var U = z.U, S = z.S, V = z.V;
    var tol = Math.max(m, n) * prec * foo, M = S.length;
    var i, Sinv = new Array(M);
    for (i = M - 1; i !== -1; i--) {
        if (S[i] > tol)
            Sinv[i] = 1 / S[i];
        else
            Sinv[i] = 0;
    }
    // return numeric.dot(numeric.dot(V,numeric.diag(Sinv)),numeric.transpose(U))

    var SinvM = [];
    for (var i = 0; i < Sinv.length; i++) {
        SinvM[i] = [];
        for (var j = 0; j < Sinv.length; j++) {
            var sinvVal = (i == j) ? Sinv[i] : 0;
            SinvM[i][j] = sinvVal;
        }
    }

    SinvM = math.matrix(SinvM);
    V = math.matrix(V);
    U = math.matrix(U);
    return math.multiply(v, math.multiply(SinvM, math.transpose(U)));
};

/*
Below Levenberg Marquart implementation is math.js port of Nathan Zimmerman's
MathsJS library (https://github.com/moradology)
[ Don't confuse with Math.JS].

MIT License (MIT)
Copyright (c) 2016 Nathan Zimmerman
Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

class comLib {
    static getMatRMS(mat) {
        return Math.sqrt(
            math.chain(mat).dotMultiply(mat).sum().done() / mat.size()[0]);
    }

    static getNumericalJacobian(inputData, fnc, params, extraData) {
        var epsL = 1e-8;
        var currParam = params.slice(0);  // Make 2 copies
        var testParams = params.slice(0);
        var currValues = math.matrix(fnc(inputData, params, extraData));

        testParams[0] += epsL;  // add pertubation
        var J = math.chain(fnc(inputData, testParams, extraData))
            .subtract(currValues)
            .multiply(1 / epsL)
            .done();
        for (var j = 1; j < testParams.length; j++) {  // Loop each column
            testParams = currParam.slice(0);             // clone orginal
            testParams[j] += epsL;                       // add pertubation
            var Ji = math.chain(fnc(inputData, testParams, extraData))
                .subtract(currValues)
                .multiply(1 / epsL)
                .done();  // J column
            J = math.concat(J, Ji)
        }
        return J;  // Matrixs obj
    }

    static numJacobian(X, modelObj) {
        var eps = 1e-8;
        var currParam = modelObj.param.slice(0);
        var currValues = modelObj.fnc(X);
        // First column of J
        modelObj.param[0] += eps;
        var J = math.chain(modelObj.fnc(X))
            .subtract(currValues)
            .multiply(1 / eps)
            .done();
        modelObj.param[0] = currParam[0];
        for (var j = 1; j < currParam.length; j++) {
            modelObj.param[j] += eps;
            var Ji = math.chain(modelObj.fnc(X))
                .subtract(currValues)
                .multiply(1 / eps)
                .done();    // J column
            J = math.concat(J, Ji);  // add to J matrix
            modelObj.param = currParam.slice(0);
        }

        return J;
    }

    static get_jacobian(datas, modelObj) {
        // Form jacobian matrix based upon data and model gradient
        if ((typeof modelObj.grad) != 'undefined') {  // if it has a jacobian
            var J = math.matrix(modelObj.grad(datas));
        } else {
            var J = comLib.numJacobian(datas, modelObj);
        }
        return J;
    }

    // getResiduals(inputData, outputData, fnc, x0, extraData);
    static getResiduals(inputData, outputData, fnc, x0, extraData) {
        var r = math.subtract(fnc(inputData, x0, extraData), outputData);
        return r;
    }

    static get_residuals(dataObj, modelObj) {
        return math.subtract(modelObj.fnc(dataObj.input), dataObj.output);
    }

    static hasConverged(costArray, ittValues, convergeTol) {
        convergeTol = (convergeTol) ? convergeTol : 0.001;
        var hasConverged = false;
        var newCost = costArray[costArray.length - 1];
        var oldCost = costArray[costArray.length - 2];
        var change = Math.abs(newCost - oldCost);

        if (newCost < convergeTol) {
            hasConverged = true;
        } else if ((typeof ittValues) != 'undefined') {
            var newCost = math.matrix(ittValues[ittValues.length - 1]);
            var oldCost = math.matrix(ittValues[ittValues.length - 2]);
            var change = comLib.getMatRMS(math.subtract(newCost, oldCost));
            if (change < convergeTol) {  // trust region expanding
                hasConverged = false;
                return hasConverged;
            }
        }
        if (change < convergeTol) {
            hasConverged = true;
        }
        return hasConverged;
    }
}

class Solvers {
    static levenbergMarquardt(dataObj, modelObj, options) {
        dataObj.output = math.matrix(dataObj.output);
        dataObj.input = math.matrix(dataObj.input);

        if (dataObj.input.size().length == 1) {
            dataObj.input = math.resize(dataObj.input, [dataObj.input.size()[0], 1]);
        }
        if (dataObj.output.size().length == 1) {
            dataObj.output =
                math.resize(dataObj.output, [dataObj.output.size()[0], 1]);
        }
        if (dataObj.input.size()[0] < dataObj.input.size()[1]) {
            dataObj.input = math.transpose(dataObj.input);
        }
        if (dataObj.output.size()[0] < dataObj.output.size()[1]) {
            dataObj.output = math.transpose(dataObj.output);
        }

        if (dataObj.output.size().length !=
            math.matrix(modelObj.fnc(dataObj.input)).size().length) {
            throw "Dimension Mismatch!!!";
        }
        if (!options) {
            options = {
                maxIter: 50,
                getCosts: false,
                getIterationValues: false,
                convergeTol: 0.001
            };
        }
        options.maxIter = options.maxIter || 50;
        var resultObj = {};
        // This is the initial guess
        resultObj.itterationValues = [math.clone(modelObj.param)];
        var r = comLib.get_residuals(dataObj, modelObj);   // initial error
        resultObj.itterationCost = [comLib.getMatRMS(r)];  // inital cost
        var currCost = comLib.getMatRMS(r);
        var newCost = currCost;
        var lamda = 0.001;

        for (var i = 0; i < options.maxIter; i++) {
            r = comLib.get_residuals(dataObj, modelObj);  // Get current error

            var J = comLib.get_jacobian(dataObj.input, modelObj);

            var H = math.chain(J).transpose().multiply(J).done();

            // var stepPart = H.add(H.diag().multiply(lamda))
            var stepPart =
                math.chain(H).add(math.multiply(math.eye(H.size()), lamda)).done();
            var step = math.chain(math.pinv(stepPart))
                .multiply(math.transpose(J))
                .multiply(r)
                .done();
            // Apply step, Update model coieficents
            modelObj.param =
                math.flatten(math.subtract(modelObj.param, math.flatten(step)))
                    .toArray();
            r = comLib.get_residuals(dataObj, modelObj);  // Get current error
            newCost = comLib.getMatRMS(r);                // store cost

            resultObj.itterationCost[i + 1] = newCost;

            if ((newCost > currCost) || (isNaN(newCost))) {  // Was it a bad step?

                lamda *= 10;  // Dampen step
                              // Revert to old model parameters
                modelObj.param = math.clone(resultObj.itterationValues[i]);
            } else {  // Was a good step
                currCost = newCost;
                lamda *= 0.1;
            }
            // Store record of model coieficents
            resultObj.itterationValues[i + 1] = math.clone(modelObj.param);
            resultObj.convergence = comLib.hasConverged(
                resultObj.itterationCost, resultObj.itterationValues,
                options.convergeTol);
            // check for convergence
            if (resultObj.convergence) {
                resultObj.iterations = i + 1;
                break;
            }
        }
        if (!options.getCosts) {
            delete resultObj.itterationCost;
        }
        if (!options.getIterationValues) {
            delete resultObj.itterationValues;
        }
        resultObj.finalCost = newCost;
        resultObj.solution = math.clone(modelObj.param);
        return resultObj;
    }

    /**
     * Returns a promise for results
     * @param {*} inputData
     * @param {*} outputData
     * @param {*} fnc
     * @param {*} Xi
     * @param {*} extraData
     * @param {*} options
     */
    static levenbergMarquardtFnc(inputData, outputData, fnc, Xi, extraData, options) {
        let wait = (ms) => new Promise(resolve => setTimeout(resolve, ms));
        inputData = math.matrix(inputData);
        outputData = math.matrix(outputData);

        // 1D, make it 2D
        if (inputData.size().length == 1) {
            inputData = math.resize(inputData, [inputData.size()[0], 1]);
        }
        if (outputData.size().length == 1) {
            outputData = math.resize(outputData, [outputData.size()[0], 1]);
        }
        // convert row to column
        if (inputData.size()[0] < inputData.size()[1]) {
            inputData = math.transpose(inputData);
        }
        if (outputData.size()[0] < outputData.size()[1]) {
            outputData = math.transpose(outputData);
        }

        if (outputData.size().length !=
            math.matrix(fnc(inputData, Xi, extraData)).size().length) {
            throw "Dimension Mismatch!!!";
        }

        options =
            options || {maxIter: 50, getCosts: false, getIterationValues: false};
        options.maxIter = options.maxIter || 50;

        var x0 = Xi.slice();
        var resultObj = {};
        resultObj.itterationValues = [x0.slice()];  // This is the initial guess
        var r = comLib.getResiduals(
            inputData, outputData, fnc, x0, extraData);    // initial error
        resultObj.itterationCost = [comLib.getMatRMS(r)];  // inital cost
        var currCost = comLib.getMatRMS(r);
        var newCost = currCost;
        var lamda = 0.001;

        const chunkLen = options.chunkLen ||
            Math.round(
                options.maxIter /
                Math.max(Math.round(inputData.size()[0] / 100, 1)));

        // last part of the solving.
        var finalFunc = function () {
            if (i < options.maxIter) {
                console.log("oops");
            }
            if (!options.getCosts) {
                delete resultObj.itterationCost;
            }
            if (!options.getIterationValues) {
                delete resultObj.itterationValues;
            }
            resultObj.finalCost = newCost;
            resultObj.solution = x0.slice();
            return resultObj;
        };

        var i = 0;
        // repititive section, maybe executed recursively
        var chunkFunc = function () {
            // breaking the main loop to chunks.
            let curChunkEnd = (Math.round(i / chunkLen) * chunkLen) + chunkLen;

            for (; i < options.maxIter && i < curChunkEnd; i++) {
                // Get current error
                r = comLib.getResiduals(inputData, outputData, fnc, x0, extraData);
                var J = comLib.getNumericalJacobian(inputData, fnc, x0, extraData);
                var H = math.multiply(math.transpose(J), J);
                // Add damping paramater
                H = math.add(H, math.multiply(math.eye(H.size()), lamda));

                var g = math.multiply(math.transpose(J), r);
                var step = math.flatten(math.multiply(math.pinv(H), g));
                // Apply step, Update model coieficents
                x0 = math.flatten(math.subtract(x0, step)).toArray();
                // Get current error
                r = comLib.getResiduals(inputData, outputData, fnc, x0, extraData);
                newCost = comLib.getMatRMS(r);  // store cost
                resultObj.itterationCost[i + 1] = newCost;

                if ((newCost > currCost)) {  // Was it a bad step?
                    lamda *= 10;               // Dampen step
                    // Revert to old model parameters
                    x0 = math.clone(resultObj.itterationValues[i]);
                } else {  // Was a good step
                    currCost = newCost;
                    lamda *= 0.1;
                }
                // Store record of model coieficents
                resultObj.itterationValues[i + 1] = x0.slice();
                resultObj.convergence = comLib.hasConverged(
                    resultObj.itterationCost, resultObj.itterationValues,
                    options.convergeTol);
                // check for convergence
                if (resultObj.convergence) {
                    resultObj.iterations = i + 1;
                    break;
                }
            }
            console.log(
                "progress: " +
                Math.round(Math.min((curChunkEnd / options.maxIter), 1) * 100) + "%");
            if (i < options.maxIter) {            // if job isnt done, call next run
                return (wait(10).then(chunkFunc));  // RECURSE!
            } else {
                // end of promise chain.
                return (0);
            }
        };
        // return the promise
        return Promise.resolve().then(chunkFunc).then(finalFunc);
    }
}

module.exports = {
    Solvers: Solvers
};

