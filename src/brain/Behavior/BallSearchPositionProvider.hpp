#pragma once

#include <list>
#include <vector>

#include "Data/BallSearchPosition.hpp"
#include "Data/BallState.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"


class Brain;

class BallSearchPositionProvider : public Module<BallSearchPositionProvider, Brain>
{
public:
  BallSearchPositionProvider(const ModuleManagerInterface& manager);

  void cycle();

private:
  /**
   * @brief Player saves all information necessary for coordinated ball search.
   * Since the Teamplayers do not include the own robot this is nice to have.
   * Will be generated from the teamplayers and the own data.
   */
  struct Player
  {
    /// the number of the player
    unsigned int playerNumber;
    /// flag to distinguish HULK robots from other team members
    bool isHULK;
    /// the pose on the field (meters, radians)
    Pose pose;
    /// If the pose is valid.
    bool isPoseValid;
    /// the position (NOT relative to the sending robot!) of the ball (meters)
    Vector2f ballPosition;
    /// time (seconds) since the robot has seen the ball
    float ballAge;
    /// if the ball filter is confident that a ball has been found
    bool isBallConfident;
    /// whether the robot is fallen
    bool fallen;
    /// whether the robot is penalized
    bool penalized;
    /// the yaw angle of this NAO's head (in rad)
    float headYaw;
    /// the position the robot is currently exploring
    Vector2f currentSearchPosition;
    /// the positions the robot is currently suggesting. (Index + 1 ^= search position for robot with player number Index + 1).
    VecVector2f suggestedSearchPositions;
    /// if the currently searched position is important
    bool isSearchPositionImportant;
    /// if the currently searched position is not relevant anymore.
    bool isSearchPositionOutdated;

    bool operator<(const Player& player) const
    {
      return playerNumber < player.playerNumber;
    }

    bool operator>(const Player& player) const
    {
      return playerNumber > player.playerNumber;
    }
  };

  /**
   * ProbabilityCell saves you how likely it is to see a ball at the given position.
   */
  struct ProbabilityCell : public DataType<ProbabilityCell>
  {
    /// How likely it is that the ball is in this cell
    float probability;
    /// The probability during last cycle.
    float oldProbability;
    /// How old the value is (in cycles)
    uint32_t age;
    /// The position if the cell's center on the field.
    Vector2f position;
    /// The indices of the cell in the map.
    Vector2i indices;
    /// If this cell is one of the searchPositions produced by this module. Also true if it is adjacent to a search pose.
    bool isSearchPositionCandidate;
    /// If this cell is too close to another search position to become itself a search position.
    bool isCloseToSearchPositionCandidate;
    /// If this cell is actually is assigned to one robot for exploration.
    bool isAssigned;

    /**
     * Resets this cell
     */
    void reset()
    {
      age = 0;
      probability = 0.01f;
      oldProbability = 0.01f;
      isSearchPositionCandidate = false;
    }

    bool operator<(const ProbabilityCell& cell) const
    {
      return probability < cell.probability;
    }

    bool operator>(const ProbabilityCell& cell) const
    {
      return probability > cell.probability;
    }

    bool operator==(const ProbabilityCell& cell) const
    {
      return indices.x() == cell.indices.x() && indices.y() == cell.indices.y();
    }

    void toValue(Uni::Value& value) const
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["probability"] << probability;
      value["age"] << age;
      value["position"] << position;
      value["indices"] << indices;
      value["isSearchPositionCandidate"] << isSearchPositionCandidate;
      value["isCloseToSearchPositionCandidate"] << isCloseToSearchPositionCandidate;
      value["isAssigned"] << isAssigned;
    }

    void fromValue(const Uni::Value& value)
    {
      value["probability"] >> probability;
      value["age"] >> age;
      value["position"] >> position;
      value["indices"] >> indices;
      value["isSearchPositionCandidate"] >> isSearchPositionCandidate;
      value["isCloseToSearchPositionCandidate"] >> isCloseToSearchPositionCandidate;
      value["isAssigned"] >> isAssigned;
    }
  };

  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<BallState> ballState_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<CycleInfo> cycleInfo_;

  /// Number of probability cells (horizontal)
  const Parameter<int> rowsCount_;
  /// Number of probability cells (vertical)
  const Parameter<int> colsCount_;
  /// The minimum distance to a ball search position (you can not find a ball when you are standing on it)
  const Parameter<float> minBallDetectionRange_;
  /// The range on which it is likely to detect a ball.
  const Parameter<float> maxBallDetectionRange_;
  /// The maximum ball age. If this age is exceeded the ball data will not be considered.
  const Parameter<float> maxBallAge_;
  /// The angle opening the fov
  Parameter<float> fovAngle_;
  /// The minimum probability to add a cell as search position
  const Parameter<float> minProbabilityToStartSearch_;
  /// The minimum probability to add a cell as a very important search position.
  const Parameter<float> minProbabilityToForceSearch_;
  /// The minimum age (in cycles) to add a cell as search position. Only used if there are not enough probable cells.
  const Parameter<int> minAgeToStartSearch_;
  /// The minimum age (in cycles) to add a cell as a very important search position.
  const Parameter<int> minAgeToForceSearch_;
  /// The weight of the kernel's core to convolve the map probabilities with.
  const Parameter<int> convolutionKernelCoreWeight_;
  /// The minimum distance between two assigned search positions. Avoids that two robots are searching adjacent cells.
  const Parameter<int> minDistanceBetweenSearchPositions_;
  /// Factor to multiply the cell's probability with if a ball was found (and is confident)
  const Parameter<float> confidentBallMultiplier_;
  /// Factor to multiply the cell's probability with if a ball was found (and is NOT confident)
  const Parameter<float> unconfidentBallMultiplier_;

  /// The position to look for a ball.
  Production<BallSearchPosition> searchPosition_;


  /// The Probability Map containing cols_ times rows_ ProbCells
  std::vector<std::vector<ProbabilityCell>> probabilityMap_;
  /// A list of pointers to all probability cells that are inside the field.
  std::list<ProbabilityCell*> probabilityList_;
  /// A list of potential searchCells
  std::vector<ProbabilityCell*> searchCellCandidates_;
  /// A list of important serachCells
  std::vector<ProbabilityCell*> importantSearchCells_;
  /// The potential search cells to send via debug.
  std::vector<ProbabilityCell> cellsToSend_;
  /// A cell that is being used if no searchPositions are available.
  ProbabilityCell dummyCell_;
  /// The search pose that was used in the last cycle.
  Pose lastSearchPose_;
  /// The final search pose later passed to searchPosition
  Pose finalSearchPose_;

  /// all players that are currently on the field (not penalized). Sorted by their player number
  std::vector<Player> activePlayers_;
  /// all players that are currently on the field (not penalized) and ready for searching the ball.
  /// Sorted by their player number.
  std::vector<Player*> explorers_;
  /// all players that are searching at a position that is not even a search candidate. Sorted by player number.
  std::vector<Player*> playersToUpdate_;

  /// Field length in m
  const float fieldLength_;
  /// Field with in m
  const float fieldWidth_;
  /// The height of one single cell
  float cellHeight_;
  /// The length of one single cell
  float cellLength_;
  /// The maximum distance the robot is able to detect a ball (squared for optimizations)
  float maxBallDetectionRangeSquared_;
  /// If the ball was seen by ANY robot last cycle.
  bool ballSeenThisCycle_;

  /**
   * Updates the map with all data available (all robot poses and ball data)
   */
  void updateMap();
  /**
   * Looks for the best cells to search the ball in.
   */
  void generateSearchCandidates();

  /**
   * Updates the probability map with the given ball data for the given pose.
   * @param pose The pose of the robot
   * @param isPoseValid Whether the pose is valid
   * @param ballPosition The absolute position the ball is being seen.
   * @param ballAge Time past since the ball was last seen by the robot (sec)
   * @param isBallConfident If the ball filter is sure that a ball was seen
   * @param headYaw The head yaw of the given robot.
   */
  void updateWithRobot(const Pose& pose, bool isPoseValid, const Vector2f& ballPosition, const float ballAge, bool isBallConfident, const float headYaw);
  /**
   * Sets the positions on which it is most likely to see a ball and pushes them into the BallSearchPosition datatype.
   */
  void updateSearchPositions();
  /**
   * @brief Function for accessing the 'adjacent' cells of the given probCell.
   * Will execute the given lambda on every cell that is adjacent to the given cell, while 'adjacent' with radius r means
   * that the indices of the other cell do not differ more than r. This will also check if any index may be out of bounds.
   * @tparam Lambda
   * @param cell The cell to get the adjacent cells for
   * @param radius The radius given in 'cell index'
   * @param lambda The expression to evaluate for each cell (EXCLUDING the given cell!)
   */
  template <typename Lambda>
  void setCellsInsideRadius(ProbabilityCell& cell, int radius, Lambda lambda)
  {
    int maxDx = (cell.indices.x() < (colsCount_() - (radius + 1))) ? radius : (colsCount_() - (2 + cell.indices.x()));
    for (int dx = (cell.indices.x() > radius) ? (0 - radius) : (0 - cell.indices.x() + 1); dx <= maxDx; dx++)
    {
      int maxDy = (cell.indices.y() < (rowsCount_() - (radius + 1))) ? radius : (rowsCount_() - (2 + cell.indices.y()));
      for (int dy = (cell.indices.y() > radius) ? (0 - radius) : (0 - cell.indices.y() + 1); dy <= maxDy; dy++)
      {
        if (dx != 0 || dy != 0)
        {
          lambda(dx, dy);
        }
      }
    }
  }
  /**
   * @brief Function for accessing the cells around the given cell on a specific radius
   * @tparam Lambda The lambda to execute
   * @param cell The cell that is used as center point
   * @param radius The radius given in 'cell index'
   * @param lambda The expression to evaluate for each cell
   */
  template <typename Lambda>
  void setCellsOnRadius(ProbabilityCell& cell, int radius, Lambda lambda)
  {
    int maxDx = (cell.indices.x() < (colsCount_() - (radius + 1))) ? radius : (colsCount_() - (2 + cell.indices.x()));
    int maxDy = (cell.indices.y() < (rowsCount_() - (radius + 1))) ? radius : (rowsCount_() - (2 + cell.indices.y()));
    for (int dx = (cell.indices.x() > radius) ? (0 - radius) : (0 - cell.indices.x() + 1); dx <= maxDx; dx++)
    {
      if (cell.indices.y() + radius <= maxDy)
      {
        lambda(dx, cell.indices.y() + radius);
      }
      if (cell.indices.y() - radius >= 1)
      {
        lambda(dx, cell.indices.y() - radius);
      }
    }
    for (int dy = (cell.indices.y() > radius) ? (0 - radius) : (0 - cell.indices.y() + 1); dy <= maxDy; dy++)
    {
      if (cell.indices.x() + radius <= maxDx)
      {
        lambda(cell.indices.x() + radius, dy);
      }
      if (cell.indices.x() - radius >= 1)
      {
        lambda(cell.indices.x() - radius, dy);
      }
    }
  }
  /**
   * Returns if the given cell is in the range of the maxBallDetectionRange from the given pose.
   * @param pose The robot's pose
   * @param cell The cell to check.
   * @return bool if the Cell is in the range.
   */
  bool isCellInBallDetectionRange(const Pose& pose, const ProbabilityCell& cell);
  /**
   * Returns if the given cell is in the field of vision (FOV) of the given robot pose.
   * http://stackoverflow.com/questions/13652518/efficiently-find-points-inside-a-circle-sector
   * @param pose the robot's pose
   * @param fovStart The Vector that opens the fov (see link)
   * @param fovEnd The Vector that closes the fov (see link)
   * @param cell the cell to check TODO
   * @return bool If the cell is in FOV.
   */
  bool isCellInFOV(const Pose& pose, const float headYaw, const ProbabilityCell& cell);
  /**
   * Returns true if it is important to look for the ball at the given position (cell).
   * @param cell
   * @return bool
   */
  inline bool isCellImportant(const ProbabilityCell& cell)
  {
    return cell.probability > minProbabilityToForceSearch_() || cell.age > static_cast<uint32_t>(minAgeToForceSearch_());
  }
  /**
   * Returns true if the given cell is interesting enough to become a search cell candidate.
   * @param cell
   * @return bool
   */
  inline bool isCellCandidate(const ProbabilityCell& cell)
  {
    return cell.probability > minProbabilityToStartSearch_() || cell.age > static_cast<uint32_t>(minAgeToStartSearch_());
  }
  /**
   * Compares the probability of two cells. Needed for sorting vectors.
   * @param first
   * @param second
   * @return True if first is more probable.
   */
  static bool isCellMoreProbable(const ProbabilityCell* first, const ProbabilityCell* second);
  /**
   * Compares the age of two cells. Needed for sorting vectors
   * @param first
   * @param second
   * @return True if first is older.
   */
  static bool isCellOlder(const ProbabilityCell* first, const ProbabilityCell* second);
  /**
   * Approximation of the time needed to walk to a given cell.
   * @param player The player to calculate the time for
   * @param cell The cell to walk to
   * @return Time in s to walk to the cell
   */
  float timeToReachCell(Player& player, ProbabilityCell& cell);
  /**
   * Send the debug output for ofa.
   */
  void sendDebug();
  /**
   * Returns the costs for the robot on currentPose to explore the cellToExplore.
   * @param currentPose The pose the robot is currently at.
   * @param currentSearchPosition The position the robot is currently searching a ball for.
   * @param cellToExplore The new cell to explore
   * @return costs for the robot to explore the cellToExplore.
   */
  int getCosts(const Pose& currentPose, const Vector2f& currentSearchPosition, const ProbabilityCell& cellToExplore);
  /**
   * Returns the cell that is closest to the given pose.
   * @param pose The pose to return a search position for.
   * @return The probability cell that is next to the given pose.
   */
  // ProbabilityCell& getSearchPositionNextTo(const Pose& pose);
  /**
   * Calculates the cell the given coordinates are in.
   * @param position the position to calculate the cell coordinates to.
   * @return std::pair of int. The coordinates of the cell.
   */
  ProbabilityCell& toCell(const Vector2f& position);
  /**
   * Deletes all Probability cells and resizes the map to the current rows_ & cols_.
   */
  void rebuildProbabilityMap();
};
