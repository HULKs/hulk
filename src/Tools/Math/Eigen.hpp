#pragma once

#include <Eigen/Dense>
#include <Eigen/StdVector>
#include <vector>

// Different vector types

using Vector2d = Eigen::Vector2d;
using Vector2f = Eigen::Vector2f;
using Vector2i = Eigen::Vector2i;
using Vector3d = Eigen::Vector3d;
using Vector3f = Eigen::Vector3f;
using Vector3i = Eigen::Vector3i;
using Vector4d = Eigen::Vector4d;
using Vector4f = Eigen::Vector4f;
using VectorXd = Eigen::VectorXd;
using VectorXf = Eigen::VectorXf;

// Different matrix types

using Matrix2d = Eigen::Matrix2d;
using Matrix2f = Eigen::Matrix2f;
using Matrix3d = Eigen::Matrix3d;
using Matrix3f = Eigen::Matrix3f;
using Matrix4f = Eigen::Matrix4f;
using Matrix4d = Eigen::Matrix4d;
using MatrixXd = Eigen::MatrixXd;
using MatrixXf = Eigen::MatrixXf;

// Different types for rotation

using Quaterniond = Eigen::Quaterniond;
using Quaternionf = Eigen::Quaternionf;
using AngleAxisf = Eigen::AngleAxisf;

using Rotation2Df = Eigen::Rotation2Df;
using Rotation2Dd = Eigen::Rotation2Dd;

// std::vector types

template <typename Scalar>
using VecVector2 =
    std::vector<Eigen::Matrix<Scalar, 2, 1>, Eigen::aligned_allocator<Eigen::Matrix<Scalar, 2, 1>>>;
template <typename Scalar>
using VecVector3 =
    std::vector<Eigen::Matrix<Scalar, 3, 1>, Eigen::aligned_allocator<Eigen::Matrix<Scalar, 3, 1>>>;

using VecVector2f = VecVector2<float>;
using VecVector2i = VecVector2<int>;
using VecVector3f = VecVector3<float>;

template <typename Scalar>
using Vector2 = Eigen::Matrix<Scalar, 2, 1>;
template <typename Scalar>
using Vector3 = Eigen::Matrix<Scalar, 3, 1>;
template <typename Scalar>
using Vector4 = Eigen::Matrix<Scalar, 4, 1>;
template <typename Scalar>
using VectorX = Eigen::Matrix<Scalar, Eigen::Dynamic, 1>;

template <typename Scalar>
using Matrix2 = Eigen::Matrix<Scalar, 2, 2>;
template <typename Scalar>
using Matrix3 = Eigen::Matrix<Scalar, 3, 3>;
template <typename Scalar>
using Matrix4 = Eigen::Matrix<Scalar, 4, 4>;
template <typename Scalar>
using MatrixX = Eigen::Matrix<Scalar, Eigen::Dynamic, Eigen::Dynamic>;
