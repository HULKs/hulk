#pragma once

#include <vector>

#include "Data/BallSearchMap.hpp"
#include "Data/BallState.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SearcherPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"

class Brain;

class SearcherPositionProvider : public Module<SearcherPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"SearcherPositionProvider"};

  /**
   * @brief SearcherPositionProvider The constructor
   * @param manager Reference to the ModuleManagerInterface (e.g. brain)
   */
  explicit SearcherPositionProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  /**
   * @brief the SearchArea struct. Divides the field into smaller areas.
   * A search area is a subset of all ProbCells. Only one player is assigned per area.
   * Also a probCell is only mapped to one searchArea. There should be none cells that
   * are not assigned to any area.
   */
  struct SearchArea
  {
    /**
     * @brief SearchArea initializes members
     */
    SearchArea()
      : voronoiSeed(Vector2f::Zero())
      , defaultPosition(Vector2f::Zero())
      , cellToExplore(nullptr)
      , assignedPlayerNumber(0)
      , assignedPlayer(nullptr){};
    /// The seed that was used to generate the area.
    Vector2f voronoiSeed;
    /// The default position to walk to if something bad happens
    Vector2f defaultPosition;
    /// All cells belonging to this area.
    std::vector<ProbCell*> cells;
    /// The cell to search
    ProbCell const* cellToExplore{nullptr};
    /// needed because the TeamPlayer pointer may become invalid in the next cycle.
    unsigned int assignedPlayerNumber{0};
    /// Pointer to the assigned player (the player to search this area)
    TeamPlayer const* assignedPlayer{nullptr};

    /**
     * @brief sets the assigned player for this search area.
     * As one (@rkost) easily forgets to set both, the assignedPlayer AND it's number,
     * this function was created.
     * @param player The player to assign.
     */
    void setAssignedPlayer(TeamPlayer const* player)
    {
      assignedPlayer = player;
      assignedPlayerNumber = static_cast<uint8_t>(player->playerNumber);
    }
  };

  /**
   * @brief MostWisePlayer the player with the oldest, continously updated map.
   * This struct saves a pointer to the player with the best map as well as it's player number to
   * check whether a player got dropped from the network / game
   */
  struct MostWisePlayer
  {
    /// pointer to the most wise player
    TeamPlayer const* player = nullptr;
    /// player number of the most wise player
    unsigned int playerNumber = 0;
    /// whether the team player pointed to is valid
    bool valid = false;
  };

  // All dependencies for this module

  const Dependency<BallSearchMap> ballSearchMap_;
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

  // TODO: These two parameters are needed by both, MapManager and PositionProvider.
  /// The minimum distance to a ball search position (you can not find a ball when you are standing
  /// on it)
  const Parameter<float> minBallDetectionRange_;
  /// The range on which it is likely to detect a ball.
  const Parameter<float> maxBallDetectionRange_;
  /// The maximum value the age of a cell can contribute to its value.
  const Parameter<float> maxAgeValueContribution_;
  /// The weight applied to a cells probability when calculating the value.
  const Parameter<float> probabilityWeight_;
  /// The voronoi seeds used to divide the field into search areas.
  const Parameter<std::vector<std::vector<Vector2f>>> voronoiSeeds_;
  /// The voronoi seeds used to divide the field into search areas during corner kick.
  const Parameter<std::vector<std::vector<Vector2f>>> cornerKickVoronoiSeeds_;

  /// The position to look for a ball.
  Production<SearcherPosition> searcherPosition_;

  /// All data of the own data stored into one TeamPlayer object.
  TeamPlayer ownTeamPlayerData_;

  /// The final search pose later passed to searchPosition
  Pose finalSearchPose_;

  /// all players that are currently on the field (not penalized). Sorted by wisdom (reliable map
  /// age)
  std::vector<const TeamPlayer*> activePlayers_;
  /// all team players that have the searcher role
  std::vector<const TeamPlayer*> searchers_;
  /// the player with the oldest, continously updated map (locally generated)
  MostWisePlayer localMostWisePlayer_;
  /// the player with the oldest, continously updated map (which we got from the player with the
  /// smallest player number)
  MostWisePlayer globalMostWisePlayer_;

  /// List of all search areas managed by this module.
  std::vector<SearchArea> searchAreas_;

  /// Field length in m
  const float fieldLength_;
  /// Field with in m
  const float fieldWidth_;

  /// indicate if search areas are cleared
  bool searchAreasCleared_ = false;

  /**
   * @brief generateOwnTeamPlayerData fills the ownTeamPlayerData object
   * This will initialize the own team player info with all information needed by the ballSearch so
   * that it can be added to the activePlayers / searchers list.
   */
  void generateOwnTeamPlayerData();
  /**
   * @brief calculates the most wise player (the player with the oldest, continously updated map)
   */
  void calculateMostWisePlayer();
  /**
   * @brief Assigns all searchers to one searchArea
   * Creates as many searchAreas as there are searchers and maps them.
   * Also does the reassignment:
   * If a previously searching robot leaves the team (penalty) or joins again, the areas will
   * be rebuild. If all searchers are available in the next cycle again it will keep the last
   * assignment.
   */
  void assignSearchAreas();
  /**
   * Assigns the search positions for every robot depending on the area that is was assigned to.
   */
  void assignSearchPositions();
  /**
   * @brief generateOwnSearchPose generates the actual search pose for this very robot.
   */
  void generateOwnSearchPose();
  /**
   * Approximation of the time needed to walk to a given position.
   * @param player The player to calculate the time for
   * @param position The position to walk to (world coordinates)
   * @return Time in seconds to walk to the position
   */
  float timeToReachPosition(const TeamPlayer& player, const Vector2f& position) const;
  /**
   * Approximation of the time needed to walk to a given cell.
   * @param player The player to calculate the time for
   * @param cell The cell to walk to
   * @return Time in seconds to walk to the cell
   */
  float timeToReachCell(const TeamPlayer& player, const ProbCell& cell) const;
  /**
   * @brief returns the value of the given cell (value increases by age and probability)
   * @param cell The cell to get the value for
   * @return float the value
   */
  float getValue(const ProbCell& cell) const;
  /**
   * Send the output for the debug tool.
   */
  void sendDebug();
  /**
   * Returns the costs for the player to search the cellToExplore.
   * @param player The player to calculate the costs for.
   * @param cellToExplore The cell to search
   * @return costs for the robot to search the cellToExplore.
   */
  float getCosts(const TeamPlayer& player, const ProbCell& cellToExplore);
  /**
   * @brief divides the field into as much areas as there are searchers on the field.
   */
  void rebuildSearchAreas();
};
