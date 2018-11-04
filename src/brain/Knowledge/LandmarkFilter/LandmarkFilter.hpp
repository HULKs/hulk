#pragma once

#include "Data/BallState.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/GoalData.hpp"
#include "Data/ImageData.hpp"
#include "Data/LandmarkModel.hpp"
#include "Data/LineData.hpp"
#include "Data/MotionState.hpp"
#include "Data/OdometryOffset.hpp"
#include "Data/PenaltySpotData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Time.hpp"

#include <set>

class Brain;

class LandmarkFilter : public Module<LandmarkFilter, Brain>, public Uni::To
{
public:
  /// the name of this module
  ModuleName name = "LandmarkFilter";
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const;
  /**
   * @brief LandmarkFilter initializes members
   * @param manager a reference to brain
   */
  LandmarkFilter(const ModuleManagerInterface& manager);
  /**
   * @brief cycle combines vision data to landmarks with filtering
   */
  void cycle();

private:
  struct GoalPost : public Uni::To
  {
    GoalPost(const Vector2f& p, TimePoint t)
      : position(p)
      , timestampLastSeen(t)
    {
    }
    /**
     * @brief toValue converts this to a Uni::Value
     * @param value the resulting Uni::Value
     */
    void toValue(Uni::Value& value) const
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["timestampLastSeen"] << timestampLastSeen;
    }

    /// the position of the goal post
    Vector2f position;
    /// the timestamp when this goal post was last percepted
    TimePoint timestampLastSeen;
  };

  struct PointCluster2D
  {
    Vector2f center;
    unsigned int clusterCount = 0;
    std::vector<float> anglesToRobotX = {};
    std::vector<size_t> lineIds = {};

    PointCluster2D(const Vector2f& cent, unsigned int count, float angleToRobX,
                   const size_t& lineId)
      : center(cent)
      , clusterCount(count)
      , anglesToRobotX({angleToRobX})
      , lineIds({lineId})
    {
    }
  };

  struct CenterPointCandidate
  {
    Vector2f point;
    float angleToRobotX = 0;
    size_t lineId = -1;

    CenterPointCandidate(const Vector2f& p, const float& angle, const size_t& parentLineId)
      : point(p)
      , angleToRobotX(angle)
      , lineId(parentLineId)
    {
    }
  };

  /**
   * @brief updateGoalPosts updates the goal posts with current goalData_ and
   *        move them according to odometry changes
   */
  void updateGoalPosts();
  /**
   * @brief assembleGoals assembles the goals from the percepted goal posts
   */
  void assembleGoals();

  /**
   * @brief filterLines filters lines which don't fulfill all the necessary criteria to be
   *        considered as a landmark
   * @param linesWithMetaData the lines and connected information that are to be filtered
   * @return the filtered line information
   */
  std::vector<LineInfo> filterLines(const std::vector<LineInfo>& linesWithMetaData);

  /**
   * @brief takes a set of lines and creates an estimate of the center circle center point
   * @param linesWithMetaData the lines and connected information used for center circle search
   */
  void findCenterCircle(const std::vector<LineInfo>& linesWithMetaData);
  /**
   * @brief filters lines for the purpose of finding a center circle
   * @param linesWithMetaData the lines and connected information that are to be filtered
   * @return filtered line infos to be used for center circle search
   */
  std::vector<LineInfo> filterLinesForCenterCircle(const std::vector<LineInfo>& linesWithMeta);
  /**
   * @brief clusters lines by how well they represent a circle
   * @param linesWithMeta lines and connected information used for clustering
   * @return the found clusters
   */
  std::vector<PointCluster2D> clusterLines(const std::vector<LineInfo>& linesWithMeta);
  /**
   * @brief Calculates the orientation of the center circle based on a line visible in it's middle
   * @param linesWithMeta lines and connected information used for finding the center line
   * orientation
   * @param candidateCluster the cluster for which's center a line orientation is to be found
   * @return a tuple containing: whether orientation was found, the id of the line used for
   *         calculating the orientation and the orientation in rad
   */
  std::tuple<bool, size_t, float>
  findCenterLineOrientation(const std::vector<LineInfo>& linesWithMeta,
                            const LandmarkFilter::PointCluster2D& candidateCluster);

  /**
   * @brief findPenaltyAreas builts a penalty area from a penalty spot and a line with set distance
   * @param relativePenaltySpotPosition the position of the detected penalty spot in robot
   * coordinates
   * @param linesWithMetaData the lines which are searched for a line that can be used for penalty
   * area construction
   * @return a vector of penalty areas
   */
  std::vector<LandmarkModel::PenaltyArea>
  findPenaltyAreas(const Vector2f& relativePenaltySpotPosition,
                   const std::vector<LineInfo>& linesWithMetaData);

  /**
   * @brief takes a set of lines and tries to find all corners and T-sections
   * @param linesWithMetaData the lines and connected information used for center circle search
   */
  void findIntersections(const std::vector<LineInfo>& linesWithMetaData);

  /**
   * @brief finds all pairs of lines that are orthogonal to each other
   * @param linesWithMetaData lines and connected information that are used for finding orthogonal
   *        lines
   * @return a vector of orthogonal line pairs
   */
  std::vector<std::pair<const LineInfo&, const LineInfo&>>
  findOrthogonalLines(const std::vector<LineInfo>& linesWithMetaData);
  /**
   * @brief constructs intersections from orthogonal line pairs
   * @param orthogonalLinePairs the line pairs from which the intersections are build
   * @return a vector of intersections
   */
  std::vector<LandmarkModel::Intersection> constructIntersections(
      const std::vector<std::pair<const LineInfo&, const LineInfo&>> orthogonalLinePairs);
  /**
   * @brief checks whether a intersection fulfills all necessary criteria
   *        also downgrades intersections when necessary (X->T->L)
   * @param[out] intersection the intersection to check
   * @return a bool which says whether the intersection passed the tests or not
   */
  bool checkIntersection(LandmarkModel::Intersection& intersection);
  /**
   * @brief tries to find the orientation of a given intersection
   * @param Intersection the intersection for which the orientation is to be found
   * @return a tuple of a bool and a float, the bool signifies if an orientation was found,
   *         the float orientation relative to the robot's x axis in radians
   */
  std::tuple<bool, float>
  findIntersectionOrientation(const LandmarkModel::Intersection& intersection);

  /**
   * @brief stores all lines (and their projectionDistances) that weren't used for
   *        landmark creation in landmarkModel_
   * @param linesWithMeta the line and connected ifnormation that is going to be stored
   */
  void saveUnusedLines(const std::vector<LineInfo>& linesWithMetaData);
  /**
   * @brief draws image with the projected center circle estimate
   */
  void sendDebugImage();

  /// if set to true, a ball will be treated as penalty spot in penalty shootout if no penalty spot
  /// was found and the ball is resting
  const Parameter<bool> ballCreatesPenaltySpotInPSO_;
  /// switches buffering of goal posts on and off
  const Parameter<bool> bufferGoalPosts_;
  /// the maximum deviation (in meters) of the distance between two goal posts
  /// to the optimal distance
  const Parameter<float> maxGoalPostDistanceDeviation_;
  /// the maximum allowed age of a goal post
  const Parameter<int> maxGoalPostAge_;
  /// the maximum allowed distance between two percepted goal posts to still allow merging
  const Parameter<float> goalPostAssociationRadius_;
  /// the maximum line lengthe relative to the circle dimension allow for circle lines
  const Parameter<float> maxLineLengthForCircleFraction_;
  /// the minimum length a line can have to still be considered part of the center circle
  const Parameter<float> minLineLengthForCircle_;
  /// the minimum amount of lines need to cluster for centre circle
  const Parameter<float> maxLineProjectionDistanceForCircle_;
  /// the maximum allowed projection distance of a line in cases with lower measurement noise
  const Parameter<uint32_t> minLineCountClusterable_;
  /// sets how many points have to be in a cluster at minimum to consider it to be the center circle
  const Parameter<uint32_t> minCountPointsInCluster_;
  /// the maximum distance a center circle candidate may have to be added to a cluster
  /// Hint: is saved squared internally to save computation
  const Parameter<float> maxDistToCircleCluster_;
  /// lines used for finding the center circle shouldnt be othogonal to each other,
  /// this sets the tolerance for which lines are still considered as orthogonal
  const Parameter<float> orthogonalTolerance_;
  /// the minimum length a line can have to be used for calculating the center circle orientation
  /// this makes sure to use the long line through the center point and not the small one
  /// Hint: is saved squared internally to save computation
  const Parameter<float> minLineLengthForCircleOrientation_;
  /// the maximum distance a enter circle candidate may have to be added to a cluster
  const Parameter<float> maxDistToCenterLineForCircleOrientation_;
  /// the minimum length a line needs to have to be used as a landmark
  const Parameter<float> minLineLength_;
  /// the maximum allowed projection distance of a line under general uncertainties
  const Parameter<float> maxLineProjectionDistance_;
  /// the maximum allowed projection distance of a line in cases with lower measurement noise
  const Parameter<float> maxLineProjectionDistanceLowNoise_;
  /// the tolerance for the distance between penalty spot and associated line for orientation
  const Parameter<float> tolerancePenaltySpotToLineDistance_;

  /// parameter that allows to toggle whether to use intersections or not
  Parameter<bool> useLineIntersections_;
  /// the minimum length that needs to overlap for an intersection of lines to
  /// be considered a T or X intersection
  const Parameter<float> minIntersectionOverlap_;
  /// The maximum distance a line can be away from the intersection point
  const Parameter<float> maxIntersectionDistance_;

  /// the squares of the corresponding parameters
  float squaredMaxDistToCircleCluster_;
  float squaredMaxDistToCenterLineForCircleOrientation_;
  float squaredMinIntersectionOverlap_;
  float squaredMaxIntersectionDistance_;

  /// the filtered ball seen by this robot (needed to fake penalty spots in PSO)
  const Dependency<BallState> ballState_;
  /// needed for debug
  const Dependency<CameraMatrix> cameraMatrix_;
  /// some information about the cycle this module is running in
  const Dependency<CycleInfo> cycleInfo_;
  /// current game controller state (for special things in PSO)
  const Dependency<GameControllerState> gameControllerState_;
  /// unfiltered goal result
  const Dependency<GoalData> goalData_;
  /// unfiltered lines
  const Dependency<LineData> lineData_;
  /// the unfiltered penaly spots as detected by vision
  const Dependency<PenaltySpotData> penaltySpotData_;
  /// the current image (only used for its timestamp)
  const Dependency<ImageData> imageData_;
  /// the field dimensions
  const Dependency<FieldDimensions> fieldDimensions_;
  /// the currently executed motion
  const Dependency<MotionState> motionState_;
  /// a reference to the odometry offset
  const Dependency<OdometryOffset> odometryOffset_;
  /// filtered landmarks
  Production<LandmarkModel> landmarkModel_;
  /// a buffer for the percepted goal posts
  std::list<GoalPost> goalPostBuffer_;
  /// the optimal distance between the center of two goal posts (according to the map)
  const float optimalGoalPostDistance_;
  /// the maximum length a line can have to still be considered part of the center circle
  float maxLineLengthForCircle_;
  /// the timestamp of the last used goal data
  TimePoint lastLineTimestamp_;
  /// the timestamp of the last used goal data
  TimePoint lastTimestamp_;
};
