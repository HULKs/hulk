import numpy as np
import math
import transforms3d


class Transforms(object):
    DTYPE = float

    @staticmethod
    def getRotMatEuler(rvec=[0, 0, 0]):
        '''
        Calculates Rotation Matrix given euler angles.
        rotEuler = rotZ * rotY * rotX
        '''
        if not (rvec[0] or rvec[1] or rvec[2]):
            return np.eye(3, dtype=Transforms.DTYPE)

        R_x = None
        R_y = None
        R_z = None
        if rvec[0]:
            R_x = np.array([[1, 0, 0],
                            [0, math.cos(rvec[0]), -math.sin(rvec[0])],
                            [0, math.sin(rvec[0]), math.cos(rvec[0])]])
        else:
            R_x = np.eye(3, dtype=Transforms.DTYPE)

        if rvec[1]:
            R_y = np.array([[math.cos(rvec[1]), 0, math.sin(rvec[1])],
                            [0, 1, 0],
                            [-math.sin(rvec[1]), 0, math.cos(rvec[1])]])
        else:
            R_y = np.eye(3, dtype=Transforms.DTYPE)

        if rvec[2]:
            R_z = np.array([[math.cos(rvec[2]), -math.sin(rvec[2]), 0],
                            [math.sin(rvec[2]), math.cos(rvec[2]), 0],
                            [0, 0, 1]])
        else:
            R_z = np.eye(3, dtype=Transforms.DTYPE)

        return np.matmul(R_z, np.matmul(R_y, R_x))

    @staticmethod
    def getHomographyEuler(rvec=[0, 0, 0], tvec=[0, 0, 0]):
        val = np.eye(4, dtype=Transforms.DTYPE)
        val[0:3, 0:3] = Transforms.getRotMatEuler(rvec)
        val[0:3, 3] = list(tvec)
        return val

    @staticmethod
    def transformHomographyEuler(rvec, tvec, data):
        return np.matmul(Transforms.getRotMatEuler(rvec), data) + tvec

    @staticmethod
    def transformHomography(transform, data):
        '''
        Must submit a 3xn matrix. or 4xn
        '''
        if data.shape[0] == 3:
            val = np.matmul(transform[0:3, 0:3], data)
        elif data.shape[0] == 4:
            val = np.matmul(transform[0:3, 0:3], data[0:3, :])
        else:
            raise ValueError("transformHomography -> Invalid dimensions!",
                             np.matrix(data).shape)
        return (val + np.array(transform[0:3, 3]).reshape(
            (3, 1))) if transform.shape[1] == 4 else val

    @staticmethod
    def getHomographyWithRotMat(rotMat, tvec):
        val = np.eye(4, dtype=Transforms.DTYPE)
        val[0:3, 0:3] = rotMat
        val[0:3, 3] = list(tvec)
        return val

    @staticmethod
    def axTransToAffine(angleaxis, tvec):
        '''
        Written in eigen - param order
        '''
        return transforms3d.affines.compose(
            tvec, transforms3d.axangles.axangle2mat(angleaxis[1:],
                                                    angleaxis[0]), np.ones(3))

    @staticmethod
    def kinematicInv(mat):
        invRot = np.transpose(mat[0:3, 0:3])
        return Transforms.getHomographyWithRotMat(
            invRot, np.matmul(invRot, -mat[0:3, 3]))
