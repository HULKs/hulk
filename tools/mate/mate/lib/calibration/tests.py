'''
Some unit tests to verify the calibration library and transform functions.

__author__ = "Darshana Adikari"
__copyright__ = "Copyright 2018, RobotING@TUHH / HULKs"
__license__ = ""
__version__ = "0.2"
__maintainer__ = "Darshana Adikari"
__email__ = "darshana.adikari@tuhh.de, darshanaads@gmail.com"
__status__ = "Alpha"
'''

import unittest

from .calibration import *
from .nao_calibration import *
from mate.lib.transforms.transforms import Transforms
import transforms3d as tr
import numpy as np


class TestTransforms(unittest.TestCase):
    def test_rotmat_euler(self):
        '''
        Rotmat in euler;

        angle axis -> angle: 2, axis: [0.26726, 0.53452, 0.80178]

        -0.314994  -0.526752   0.789495
        0.931361 -0.0115373   0.363896
        -0.18258   0.849933   0.494225
        '''
        ai = 1.0440968737568794
        aj = 0.18360980455612225
        ak = 1.8969255909107627

        i = transforms3d.euler.euler2mat(ai, aj, ak, axes='sxyz')

        j = np.matrix([[-0.31499349, -0.52675319,
                        0.78949996], [0.93136657, -0.01153345, 0.36390011],
                       [-0.18257988, 0.84994003, 0.49423327]])

        k = Transforms.getRotMatEuler([ai, aj, ak])
        self.assertTrue(np.allclose(j, j))
        self.assertTrue(np.allclose(j, k))

    def test_angleExisStuff(self):
        '''
        angle axis -> angle: 2, axis: [0.26726, 0.53452, 0.80178]

        Compare angle-axis conversion with Eigen output
        '''
        angle_axis = [2, 0.26726, 0.53452, 0.80178]
        translate = [10, 20, 30]

        orig_ = transforms3d.axangles.axangle2mat(angle_axis[1:],
                                                  angle_axis[0])

        angle_axis_translate = Transforms.axTransToAffine(
            angle_axis, translate)
        j = np.matrix([[-0.31499349, -0.52675319, 0.78949996, translate[0]], [
            0.93136657, -0.01153345, 0.36390011, translate[1]
        ], [-0.18257988, 0.84994003, 0.49423327, translate[2]], [0, 0, 0, 1]])

        self.assertTrue(np.allclose(angle_axis_translate, j))

    def test_get_homography_euler(self):
        i = Transforms.getHomographyEuler(
            [1.0440968737568794, 0.18360980455612225, 1.8969255909107627],
            [0, 0, 0])

        j = np.matrix([[-0.31499349, -0.52675319, 0.78949996,
                        0], [0.93136657, -0.01153345, 0.36390011, 0],
                       [-0.18257988, 0.84994003, 0.49423327, 0], [0, 0, 0, 1]])
        self.assertTrue(np.allclose(i, j))


class TestNaoCamProps(unittest.TestCase):
    input = [[
        448.8947792632463, 448.89261938051357, 448.89045949778085,
        417.10884066882824, 417.10668078609547, 417.10452090336275,
        385.32290207441014, 385.32074219167737, 385.31858230894466,
        353.5369634799921, 353.5348035972593, 353.53264371452656,
        321.751024885574, 321.7488650028412, 321.74670512010846,
        289.9650862911559, 289.9629264084232, 289.9607665256904
    ], [
        49.98409332285184, -0.015906562414752908, -50.01590644768135,
        49.98748054433063, -0.012519340935958528, -50.01251922620256,
        49.990867765809426, -0.009132119457164148, -50.00913200472377,
        49.99425498728822, -0.005744897978369768, -50.00574478324497,
        49.99764220876702, -0.0023576764995753875, -50.002357561766175,
        50.00102943024581, 0.001029544979218991, -49.998970340287386
    ], [
        -120.57147369092206, -120.56886442891874, -120.56625516691543,
        -81.97543637880136, -81.97282711679804, -81.97021785479473,
        -43.37939906668066, -43.37678980467729, -43.374180542674026,
        -4.783361754559905, -4.780752492556587, -4.778143230553269,
        33.812675557560794, 33.81528481956411, 33.81789408156743,
        72.4087128696815, 72.41132213168487, 72.41393139368813
    ]]
    measure = [[
        257.2973327636719, 318.7681884765625, 382.69390869140625,
        250.78036499023438, 319.29107666015625, 387.9034423828125,
        245.1998291015625, 319.517333984375, 393.7038269042969,
        238.0653533935547, 319.4933166503906, 400.7959899902344,
        229.480224609375, 319.5187072753906, 409.5124206542969,
        218.767822265625, 319.5057373046875, 420.1414489746094
    ], [
        409.733642578125, 409.6559143066406, 409.52728271484375,
        369.65740966796875, 369.4448547363281, 369.4945068359375,
        323.50140380859375, 323.48492431640625, 323.4820556640625,
        269.5050964355469, 269.4952392578125, 269.50799560546875,
        202.52102661132812, 202.51495361328125, 202.51925659179688,
        120.49624633789062, 120.49151611328125, 120.51859283447266
    ]]
    gen_octave = [[
        257.661, 320.020, 382.379, 252.906, 320.017, 387.128, 247.366, 320.013,
        392.661, 240.831, 320.009, 399.188, 233.004, 320.004, 407.006, 223.461,
        319.998, 416.537
    ], [
        390.965, 390.962, 390.959, 350.461, 350.458, 350.455, 303.275, 303.272,
        303.268, 247.605, 247.600, 247.596, 180.934, 180.929, 180.925, 99.648,
        99.641, 99.635
    ]]
    intrinsic_bottom = [[559.85, 562.05], [320.0, 240.0]]

    def test_extrinsic_matrix(self):
        val = [-1.0905224730e+0, 7.8262771790e-1, -5.1615916270e-1]

        i = NaoCamProps.getExtrinsicMat(val)
        j = np.matrix([[0.616687, 0.349953,
                        0.705145], [-0.771926, 0.0931824, 0.628846],
                       [0.154359, -0.932121, 0.327603]])

        self.assertTrue(np.allclose(i, j))

    def test_camera_to_pixel(self):
        i = NaoCamProps.cameraToPixel(TestNaoCamProps.intrinsic_bottom[0],
                                      TestNaoCamProps.intrinsic_bottom[1],
                                      np.matrix(TestNaoCamProps.input))
        print("extrinsic: \n", i, "\n\n", np.matrix(TestNaoCamProps.input))

        self.assertTrue(np.allclose(i, np.matrix(TestNaoCamProps.gen_octave)))

if __name__ == '__main__':
    unittest.main()
