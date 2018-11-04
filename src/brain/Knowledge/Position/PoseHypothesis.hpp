#pragma once

#include <cmath>

#include "Data/FieldDimensions.hpp"
#include "Data/LandmarkModel.hpp"
#include "Framework/Module.hpp"
#include "Knowledge/Position/FieldInfo.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/StateEstimation/ProjectionMeasurementModel.hpp"
#include "Tools/Storage/UniValue/UniConvertible.hpp"

#include "UKFPose2D.hpp"

class PoseHypothesis : public UKFPose2D
{
public:
  /**
   * @brief PoseHypothesis initializes the members of the PositionKnowledge
   * @param module a reference to the module base that owns this PoseHyspothesis
   * @param fieldDimensions some information about the dimensions of the field
   * @param fieldInfo some information about the position of features on the field
   */
  PoseHypothesis(const ModuleBase& module, const FieldDimensions& fieldDimensions,
                 const FieldInfo& fieldInfo);
  /**
   * @brief reset resets UKFPose to given mean and covariance
   * @param poseMean the state mean to be set
   * @param poseCov the diagonal entries of the intial cov matrix
   * @param error the error of this pose
   * @param id the id of this hypothesis
   */
  void reset(const Pose& poseMean, const Vector3f& poseCov, float error, int id);
  /**
   * @brief operator < compares this hypothesis to another one (by error)
   * @param other the PoseHypothesis to compare with
   * @return true if this hypothesis is worse (larger error)
   */
  bool operator<(const PoseHypothesis& other) const
  {
    const float myError = getMeanEvalError();
    const float otherError = other.getMeanEvalError();

    if (myError < 0.f)
    {
      return true;
    }
    else if (otherError < 0.f)
    {
      return false;
    }
    else
    {
      return this->getMeanEvalError() > other.getMeanEvalError();
    }
  }
  /**
   * @brief operator < compares this hypothesis to another one (by error)
   * @param other the PoseHypothesis to compare with
   * @return true if this hypothesis is better (smaller error)
   */
  bool operator>(const PoseHypothesis& other) const
  {
    const float myError = getMeanEvalError();
    const float otherError = other.getMeanEvalError();

    if (myError < 0.f)
    {
      return false;
    }
    else if (otherError < 0.f)
    {
      return true;
    }
    else
    {
      return this->getMeanEvalError() < other.getMeanEvalError();
    }
  }
  /**
   * @brief getMeanEvalError getter for the meanEvalError_ of this hypothesis
   * @return the current meanEvalError_ of this hypothesis
   */
  float getMeanEvalError() const;
  /**
   * @brief getID return the id of this hypothesis
   * @return the id of this hypothesis
   */
  int getID() const;
  /**
   * @brief evaluate evaluates the quality of this pose hypothesis
   * @param setOfRelativeLines a set of lines in robot coordinates used to validate the hypothesis
   */
  void evaluate(const std::vector<Line<float>>& setOfRelativeLines);
  /**
   * @brief isInNeighbourhood compares this hypothesis to another one
   * @param other the other hypothesis this hypothesis is to be compared with
   * @param eps a 2D-vector allowed position and angle tolerance
   * @return true if other is assumed to be the same hypothesis
   */
  bool isInNeighbourhood(const PoseHypothesis& other, const Vector2f& eps) const;
  /**
   * @brief mirror mirrors this hypothesis to the symmetric hypothesis on the oppnents half
   */
  void mirror();
  /**
   * @brief updateWithSetOfLines updates this hypothesis with a set of lines, respecting the
   * topological context
   * @param setOfRelativeLines the set of relative lines, this hypothesis is to be updated with
   * @param cam2ground the camera matrix this projection was calculated with
   */
  void updateWithSetOfLines(const std::vector<Line<float>>& setOfRelativeLines,
                            const KinematicMatrix& cam2ground,
                            const bool searchCirceInLines = false);
  /**
   * @brief updateWithCenterCircle updates this hypothesis with a center circle. This will either
   * perform a fieldmark update (if no orientation information is available), or a full pose update
   * @param centerCircle a center circle in robot coordinates (with or without valid
   * @param cam2ground the camera matrix this projection was calculated with
   * @param ignoreCirclePerceptsNearGoalSupport if set to true, circle percepts that would be near
   * in field coordinates goal support area when projected from the hypothesis will be ignored
   * orientation)
   */
  void updateWithCenterCircle(const LandmarkModel::CenterCircle& centerCircle,
                              const KinematicMatrix& cam2ground,
                              const bool ignoreCirclePerceptsNearGoalSupport);
  /**
   * @brief updateWithPenaltyArea updates this hypothesis with a penalty area. This will
   * perform a fieldmark update
   * @param relativePenaltyArea the relative position (and orientation) of the penalty area.
   * Note: The position is defined by the penalty spot. The orientation is zero when standing in
   * front of the penalty area / goal.
   * @param cam2ground the camera matrix this projection was calculated with
   * @param ignorePenaltyAreasWithoutOrientation ignore penalty areas that don't provide any
   * orientation information
   */
  void updateWithPenaltyArea(const LandmarkModel::PenaltyArea& relativePenaltyArea,
                             const KinematicMatrix& cam2ground,
                             const bool ignorePenaltyAreaWithoutOrientation);
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  virtual void toValue(Uni::Value& value) const;

protected:
  struct AssociatedLine
  {
    const Line<float>* ptrToLine = NULL;
    float error = 1337.f;
  };

  struct PointCluster2D
  {
    Vector2f center;
    unsigned int clusterCount;

    PointCluster2D(const Vector2f& cent, unsigned int count)
      : center(cent)
      , clusterCount(count)
    {
    }
  };

  /// the base maximum angle that is allowed for line association
  Parameter<float> baseAssociationAngleThreshold_;
  /// the maximum deviation of the center points
  const Parameter<float> maxDistToCircleClusterSquared_;
  /// the minimum number of points to form a circle
  const Parameter<int> minCircleClusterCount_;
  /// the gain of innovation for the error low pass filter
  const Parameter<float> evalLowPassGain_;
  /// the fraction of the evalError determined by reciprocal assocation fraction
  const Parameter<float> evalAssocationFraction_;
  /// the base variance of measurements (added to every error propagation)
  Parameter<Vector3f> measurementBaseVariance_;
  /// the basic deviation of the camera matrix roll poitch an yaw in deg
  Parameter<Vector3f> cameraRPYDeviation_;

  /// the measurement model to estimate point covariances
  ProjectionMeasurementModel projectionMeasurementModel_;
  /// some information about the dimensions of the field
  const FieldDimensions& fieldDimensions_;
  /// some information about the position of landmarks on the field
  const FieldInfo& fieldInfo_;
  /// the error of this pose resulting form the evaluation
  float meanEvalError_;
  /// a vector for the centerPontCandidates of this cycle
  std::vector<Vector2f> centerPointCandidates_;
  /// the id to identify this hypothesis
  int id_;

  /**
   * @brief findAssociatedLine finds the associated field line for a percepted line (by finding the
   * best/nearest fit)
   * @param relativeLine a percepted line in relative coordinates
   * @param referencePose the (assumed) pose, the relativeLine was percepted from
   * @param pruneByDistanceThreshold if set to false, all line will be associated
   * @return a pointer to the associated line (in absolute coordinates)
   */
  AssociatedLine findAssociatedLine(const Line<float>& relativeLine, const Vector3f& referencePose,
                                    const bool pruneByDistanceThreshold = true) const;
  /**
   * @brief computePose1DFromLine calculates an absolute 1D Pose (x or y coordinate, orientation)
   * from a given (percepted) absolute line and the corresponding associatedLine in absolute
   * coordinates
   * @param relativeLine a reference to the percepted line in absolute coordinates
   * @param associatedLine a reference to the absolute field line associated with the percepted line
   * @param referencePose a reference to the reference pose the relative line was seen from
   * @return an Vector2f (pose x, y, orientation) of a onedimensional pose update
   */
  Vector2f computePose1DFromLine(const Line<float>& relativeLine, const Line<float>& associatedLine,
                                 const Vector3f& referencePose) const;
  /**
   * @brief lineSensorUpdate updated the hypothesis with a percepted line (in robot coordinates) by
   * associatingit with a field line and performing a pose1DSensorUpdate (Parent class) with the
   * retrieved information
   * @param relativeLine a percepted line in robot coordinates
   * @param refPose the pose to be used for the line association
   * in field coordinates
   * @param cam2ground the camera matrix this projection was calculated with
   */
  void lineSensorUpdate(const Line<float>& relativeLine, const Vector3f& refPose,
                        const KinematicMatrix& cam2ground);
  /**
   * @brief computeRelativeCenterCirclePositionFromTangent calculates the relative center circle
   * candidates from a given line that is believed to be tangential to the circle and adds them
   * to centerCircleCandidates_
   * @param relativeLine a reference to the percepted line in absolute coordinates
   * @param fieldDimensions a reference to the field dimensions
   */
  void generateCenterCircleCandatesFromTanget(const Line<float>& relativeLine);
  /**
   * @brief angularMetric computes a kind of angule difference between two direction vectors
   * @param a a vector representing a direction
   * @param b a vector representing a direction
   * @return a value in [0, 1] that corresponds to the angle difference (smaller is better)
   */
  float angularMetric(const Vector2f& a, const Vector2f& b) const;
  /**
   * @brief http://stackoverflow.com/a/1501725/2169988 (find the shortest distance between a point
   * and a line segment)
   * @param line the line to get the distance to
   * @param point a point which distance to a line is to be computed
   * @return shortest distance between point and line
   */
  float distPointToLineSegment(const Line<float>& line, const Vector2f& point) const;
  /**
   * @brief isFieldLineAlongAxis method to find out whether a fieldLine is aligned with one of the
   * axis (x, y)
   * @param fieldLine a field line to check for axis alignment
   * @return true if fieldLine is aligned with one axis
   */
  bool isFieldLineAlongAxis(const Line<float>& fieldLine) const;
  /**
   * @brief absoluteFromRelativeLine calculates an absolute line from a relative line when given a
   * reference pose
   * @param relativeLine a Line in realtive coordinates
   * @param referencePose the pose in whichs coordinate system the relativeLine was given
   * @return the Line in absolute coordinates
   */
  Line<float> absoluteFromRelativeLine(const Line<float>& relativeLine,
                                       const Vector3f& referencePose) const;
  /**
   * @brief computePoseCovFromOrientationFeature calculates the covariance of a feature that
   * contains information about all three state components (x,y,alpha)
   * @param relativeFeaturePosition the relative position of the detected feature
   * @param associatedOrientation the orientation of the pose associated with this measurement after
   * full convergence (infinit gain update)
   * @param cam2ground the relative 3D pose of the camera with respect to the ground
   * @return the resulting covariance estimation
   */
  Eigen::Matrix3f computePoseCovFromFullPoseFeature(const Vector2f& relativeFeaturePosition,
                                                    const float associatedOrientation,
                                                    const KinematicMatrix& cam2ground) const;
  /**
   * @brief computeCircleCenterFromCandidates computes the relative position of the center circle
   * form a set of given candidates
   * @param centerPointCandidates a reference to the vector of circle center candidates in relative
   * coordinates
   * @param a reference to the result
   * @return true if valid cluster was written to result
   */
  bool computeCircleCenterFromCandidates(const std::vector<Vector2f>& centerPointCandidates_,
                                         Vector2f& result) const;
};
