#pragma once

#include "UKF.hpp"


template <int n>
UKF<n>::UKF()
  : stateMean_(VectorN::Zero())
  , stateCov_(MatrixN::Identity())
  , sigmaPoints_()
{
}

template <int n>
UKF<n>::UKF(const VectorN& mean, const MatrixN& cov)
  : stateMean_(mean)
  , stateCov_(cov)
  , sigmaPoints_()
{
}

template <int n>
const typename UKF<n>::VectorN& UKF<n>::getStateMean() const
{
  return stateMean_;
}

template <int n>
void UKF<n>::reset(const VectorN& meanInit, const MatrixN& covInit)
{
  stateMean_ = meanInit;
  stateCov_ = covInit;
}

template <int n>
void UKF<n>::generateSigmaPoints()
{
  // sample 2n+1 points along sigma contour using cholesky decomposition
  Eigen::LLT<MatrixN> stateCovCholesky(stateCov_);
  const MatrixN covSqrt = stateCovCholesky.matrixL();

  sigmaPoints_[0] = stateMean_;
  for (unsigned int i = 1; i < 2 * n + 1; i++)
  {
    const int rootSign = i % 2 ? -1 : 1;
    sigmaPoints_[i] = stateMean_ + rootSign * covSqrt.col((i - 1) / 2);
  }
}

template <int n>
void UKF<n>::predictWithAWGN(
    std::function<VectorN(const VectorN& sigmaPoint)> stateSpacePredictFunction,
    const MatrixN& processNoise)
{
  // generate the sigma points for the unscented transformation
  generateSigmaPoints();
  // Propagate each sigma point through the nonlinear predict function
  std::transform(sigmaPoints_.begin(), sigmaPoints_.end(), sigmaPoints_.begin(),
                 stateSpacePredictFunction);

  std::tie(stateMean_, stateCov_) = computeStatistics(sigmaPoints_);
  stateCov_ += processNoise;
  assert(stateCov_ == stateCov_.transpose());
}


template <int n>
template <int nZ>
void UKF<n>::updateWithAWGN(const Eigen::Matrix<float, nZ, 1>& observation,
                            const Eigen::Matrix<float, nZ, nZ>& observationNoise,
                            std::function<Eigen::Matrix<float, nZ, 1>(const VectorN& sigmaPoint)>
                                predictObservationFromStateSpace)
{
  // generate the sigmaPoints_ for the UC predict
  generateSigmaPoints();
  // Propagate each sigma point through the nonlinear observation function
  std::array<Eigen::Matrix<float, nZ, 1>, numOfSigmaPoints> predictedObservations;
  std::transform(sigmaPoints_.begin(), sigmaPoints_.end(), predictedObservations.begin(),
                 predictObservationFromStateSpace);
  // compute statistics of predicted observation
  Eigen::Matrix<float, nZ, 1> predictedObservationsMean;
  Eigen::Matrix<float, nZ, nZ> predictedObservationsCov;
  std::tie(predictedObservationsMean, predictedObservationsCov) =
      computeStatistics(predictedObservations);
  // Pxz - corss-covariance matrix of sigma points and predicted observation
  const auto predictedObservationCrossCov = computeCrossCovariance(
      sigmaPoints_, stateMean_, predictedObservations, predictedObservationsMean);
  // compute the kalman gain
  const auto kalmanGain =
      predictedObservationCrossCov * (predictedObservationsCov + observationNoise).inverse();
  // residuum
  const auto residuum = observation - predictedObservationsMean;
  // a posteriori state estimate
  stateMean_ += kalmanGain * residuum;
  stateCov_ -= kalmanGain * predictedObservationCrossCov.transpose();
  fixCovariance(stateCov_);
}

template <int n>
template <int dim>
void UKF<n>::fixCovariance(Eigen::Matrix<float, dim, dim>& cov) const
{
  auto covTranspose = cov.transpose();
  cov = 0.5f * (covTranspose + cov);
}

template <int n>
template <size_t size, int dim>
Gauss<dim>
UKF<n>::computeStatistics(const std::array<Eigen::Matrix<float, dim, 1>, size>& elements) const
{
  static_assert(size > 0, "size was 0 or smaller in statistics calculation");
  Eigen::Matrix<float, dim, 1> sum = Eigen::Matrix<float, dim, 1>::Zero();
  for (auto& element : elements)
  {
    sum += element;
  }
  auto mean = sum / size;
  Eigen::Matrix<float, dim, dim> cov = Eigen::Matrix<float, dim, dim>::Zero();
  for (auto& element : elements)
  {
    auto diff = element - mean;
    for (unsigned int i = 0; i < diff.size(); i++)
    {
      cov.col(i) += diff * diff(i);
    }
  }
  cov *= 0.5f;

  fixCovariance(cov);
  return {mean, cov};
}

template <int n>
template <size_t size, int dimA, int dimB>
Eigen::Matrix<float, dimA, dimB>
UKF<n>::computeCrossCovariance(const std::array<Eigen::Matrix<float, dimA, 1>, size>& setA,
                               const Eigen::Matrix<float, dimA, 1>& meanA,
                               const std::array<Eigen::Matrix<float, dimB, 1>, size>& setB,
                               const Eigen::Matrix<float, dimB, 1>& meanB) const
{
  Eigen::Matrix<float, dimA, dimB> crossCov = Eigen::Matrix<float, dimA, dimB>::Zero();
  for (unsigned int i = 0; i < size; i++)
  {
    const auto diffA = setA[i] - meanA;
    const auto diffB = setB[i] - meanB;
    crossCov += diffA * diffB.transpose();
  }
  return crossCov * 0.5f;
}
