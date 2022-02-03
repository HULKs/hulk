#pragma once

#include <algorithm>
#include <tuple>

#include "Tools/Math/Eigen.hpp"

template <int dim>
using Gauss = std::tuple<Eigen::Matrix<float, dim, 1>, Eigen::Matrix<float, dim, dim>>;


template <int n>
class UKF
{
public:
  using VectorN = Eigen::Matrix<float, n, 1>;
  using MatrixN = Eigen::Matrix<float, n, n>;
  using GaussN = Gauss<n>;

  /**
   * @brief UKF An Unscented Kalman filter for an n dimensional state vector.
   */
  UKF();
  /**
   * @brief UKF
   * @param mean the initial mean of the estimation
   * @param cov the initial cov of the estimation
   */
  UKF(const VectorN& mean, const MatrixN& cov);
  /**
   * @brief getStateMean a getter for the current state mean
   * @return the current stateMean
   */
  const VectorN& getStateMean() const;
  /**
   * @brief reset resets the state (mean and cov) to given values;
   * @param meanInit the new state mean
   * @param meanInit the new state covariance
   */
  void reset(const VectorN& meanInit, const MatrixN& covInit);
  /**
   * @brief generateSigmaPoints generates the according number of sigma points along the sigma
   * contour
   */
  virtual void generateSigmaPoints();
  /**
   * @brief predict the UC-predict mapping the old state onto a new state.
   * @param args some external input (e.g. to account non-linearity)
   * @return the transformed state distribution
   */
  void predictWithAWGN(std::function<VectorN(const VectorN& sigmaPoint)> stateSpacePredictFunction,
                       const MatrixN& processNoise);
  /**
   * @brief update the UC-update correcting the state with some external knowledge. Use this for
   * nonlinear observation functions. Note: For linear observation functions, this is overkill. Use
   * classical kalman update instead.
   * @param observation the nZ dimensional observation
   * @param observationNoise the covariance of the of the observation
   * @param predictObservationFromStateSpace a mapping from the state space to the space of
   * observation (which observation would a given state make)
   */
  template <int nZ>
  void updateWithAWGN(const Eigen::Matrix<float, nZ, 1>& observation,
                      const Eigen::Matrix<float, nZ, nZ>& observationNoise,
                      std::function<Eigen::Matrix<float, nZ, 1>(const VectorN& sigmaPoint)>
                          predictObservationFromStateSpace);

protected:
  /// the n-dimensional mean of the state random variable
  VectorN stateMean_;
  /// the nxn-dimensional covariance of the state variable
  MatrixN stateCov_;
  /// the number of points sampled from the state space to approximate the distribution
  static const constexpr unsigned int numOfSigmaPoints = 2 * n + 1;
  /// the sigma points used for approximation of the state gaussian
  std::array<VectorN, numOfSigmaPoints> sigmaPoints_;

  /**
   * @brief fixCovariance ensures that a given cov is symmetric
   * @param cov a reference to the covariance matrix to be fixed
   */
  template <int dim>
  void fixCovariance(Eigen::Matrix<float, dim, dim>& cov) const;
  /**
   * @brief computeStatistics computes the statistic (mean and covariance) of a given array of
   * dim-dimensional Eigen::Vectors
   * @param elements the array of elements to compute the statistics fro
   * @return the gaussian distribution of the set
   */
  template <size_t size, int dim>
  Gauss<dim>
  computeStatistics(const std::array<Eigen::Matrix<float, dim, 1>, size>& elements) const;

  template <size_t size, int dimA, int dimB>
  Eigen::Matrix<float, dimA, dimB>
  computeCrossCovariance(const std::array<Eigen::Matrix<float, dimA, 1>, size>& setA,
                         const Eigen::Matrix<float, dimA, 1>& meanA,
                         const std::array<Eigen::Matrix<float, dimB, 1>, size>& setB,
                         const Eigen::Matrix<float, dimB, 1>& meanB) const;
};

#include "Tools/StateEstimation/UKF_Impl.hpp"
