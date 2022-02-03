#include "Brain/Behavior/SearcherPositionProvider.hpp"
#include "Data/PlayingRoles.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/HungarianMethod.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Math/Range.hpp"

SearcherPositionProvider::SearcherPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , ballSearchMap_(*this)
  , gameControllerState_(*this)
  , playerConfiguration_(*this)
  , playingRoles_(*this)
  , teamPlayers_(*this)
  , ballState_(*this)
  , robotPosition_(*this)
  , bodyPose_(*this)
  , teamBallModel_(*this)
  , fieldDimensions_(*this)
  , jointSensorData_(*this)
  , cycleInfo_(*this)
  , minBallDetectionRange_(*this, "minBallDetectionRange", [] {})
  , maxBallDetectionRange_(*this, "maxBallDetectionRange", [] {})
  , maxAgeValueContribution_(*this, "maxAgeValueContribution", [] {})
  , probabilityWeight_(*this, "probabilityWeight", [] {})
  , voronoiSeeds_(*this, "voronoiSeeds", [] {})
  , cornerKickVoronoiSeeds_(*this, "cornerKickVoronoiSeeds", [] {})
  , searcherPosition_(*this)
  , fieldLength_(fieldDimensions_->fieldLength)
  , fieldWidth_(fieldDimensions_->fieldWidth)
{
  rebuildSearchAreas();
}

void SearcherPositionProvider::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time");

    // Updating the map while not playing may make things worse.
    if (gameControllerState_->gameState != GameState::PLAYING)
    {
      // While not playing the search pose is set to a position near the center of the field.
      searcherPosition_->pose = Pose(-0.5f, 0.f);
      searcherPosition_->searchPosition = {-0.5f, 0.f};
      for (auto& suggestion : searcherPosition_->suggestedSearchPositions)
      {
        suggestion = {-0.5f, 0.f};
      }
    }
    else
    {
      activePlayers_.clear();
      searchers_.clear();

      generateOwnTeamPlayerData();

      // Add myself to active players
      if (!ownTeamPlayerData_.penalized && !ownTeamPlayerData_.fallen)
      {
        activePlayers_.emplace_back(&ownTeamPlayerData_);
      }

      // Add all other team players to active players
      for (const auto& teamPlayer : teamPlayers_->players)
      {
        if (teamPlayer.penalized || teamPlayer.fallen)
        {
          continue;
        }
        activePlayers_.emplace_back(&teamPlayer);
      }

      // No reason to continue if there is no active player
      if (activePlayers_.empty())
      {
        return;
      }

      // Sort player by wisdom to find the player with the oldest, continuously updated map.
      std::sort(activePlayers_.begin(), activePlayers_.end(),
                [](const TeamPlayer* a, const TeamPlayer* b) {
                  return (a->timestampBallSearchMapUnreliable == b->timestampBallSearchMapUnreliable
                              ? a->playerNumber < b->playerNumber
                              : a->timestampBallSearchMapUnreliable <
                                    b->timestampBallSearchMapUnreliable);
                });

      // Find all active players that have the searcher role and add them to the
      // searchers list
      for (auto& teamPlayer : activePlayers_)
      {
        // Check if the robot is ready to search for the ball (he may exclude himself)
        if (teamPlayer->currentlyPerformingRole == PlayingRole::SEARCHER)
        {
          searchers_.emplace_back(teamPlayer);
        }
      }

      calculateMostWisePlayer();
      assert(localMostWisePlayer_.valid && "localMostWisePlayer needs to be valid");
      assert(globalMostWisePlayer_.valid && "globalMostWisePlayer needs to be valid");
      ownTeamPlayerData_.mostWisePlayerNumber = localMostWisePlayer_.playerNumber;

      if (gameControllerState_->setPlay == SetPlay::CORNER_KICK && !searchAreasCleared_)
      {
        searchAreas_.clear();
        searchAreasCleared_ = true;
      }

      if (gameControllerState_->setPlay != SetPlay::CORNER_KICK && searchAreasCleared_)
      {
        searchAreasCleared_ = false;
      }

      if (!searchers_.empty())
      {
        assignSearchAreas();
        assignSearchPositions();
      }

      // Do not calculate a search pose for a robot that is penalized or not searcher
      if (playingRoles_->role == PlayingRole::SEARCHER)
      {
        generateOwnSearchPose();
      }
    }
  }
  sendDebug();
}

void SearcherPositionProvider::generateOwnTeamPlayerData()
{
  // create a teamPlayer object for oneself.
  ownTeamPlayerData_.playerNumber = playerConfiguration_->playerNumber;
  ownTeamPlayerData_.isHULK = true;
  ownTeamPlayerData_.pose = robotPosition_->pose;
  ownTeamPlayerData_.ballPosition = ballState_->position;
  ownTeamPlayerData_.timeWhenBallWasSeen = ballState_->timeWhenLastSeen;
  ownTeamPlayerData_.fallen = bodyPose_->fallen;
  ownTeamPlayerData_.penalized = gameControllerState_->penalty != Penalty::NONE;
  ownTeamPlayerData_.headYaw = jointSensorData_->angles[Joints::HEAD_YAW];
  ownTeamPlayerData_.currentSearchPosition = finalSearchPose_.position();
  ownTeamPlayerData_.currentlyPerformingRole = playingRoles_->role;
}

void SearcherPositionProvider::calculateMostWisePlayer()
{
  // check if the most wise player was dropped out of the network / game and find the leader from
  // who we get the (global)mostWisePlayer
  localMostWisePlayer_.valid = false;
  globalMostWisePlayer_.valid = false;
  unsigned int smallestPlayerNumber = std::numeric_limits<unsigned int>::max();
  for (auto& teamPlayer : activePlayers_)
  {
    if (teamPlayer->playerNumber == localMostWisePlayer_.playerNumber)
    {
      localMostWisePlayer_.player = teamPlayer;
      localMostWisePlayer_.valid = true;
    }
    if (teamPlayer->playerNumber < smallestPlayerNumber)
    {
      smallestPlayerNumber = teamPlayer->playerNumber;
    }
  }

  if (!localMostWisePlayer_.valid)
  {
    // active players are sorted by wisdom. Persist the most wise player.
    localMostWisePlayer_.player = activePlayers_[0];
    localMostWisePlayer_.playerNumber = localMostWisePlayer_.player->playerNumber;
    localMostWisePlayer_.valid = true;
  }
  else
  {
    if (Hysteresis::smallerThan(activePlayers_[0]->timestampBallSearchMapUnreliable,
                                localMostWisePlayer_.player->timestampBallSearchMapUnreliable, 5s,
                                false))
    {
      localMostWisePlayer_.player = activePlayers_[0];
      localMostWisePlayer_.playerNumber = localMostWisePlayer_.player->playerNumber;
      localMostWisePlayer_.valid = true;
    }
  }

  // find the most wise player that is suggested by the player with the smallest player number
  for (auto& teamPlayer : activePlayers_)
  {
    if (teamPlayer->playerNumber == smallestPlayerNumber)
    {
      // we found the guy with the smallest player number (the leader that tells us the player we
      // should use the data from)
      globalMostWisePlayer_.playerNumber = teamPlayer->mostWisePlayerNumber;
      // search for the actual most wise player suggested by the leader
      const auto mostWisePlayer =
          std::find_if(activePlayers_.begin(), activePlayers_.end(), [this](const auto& player) {
            return globalMostWisePlayer_.playerNumber == player->playerNumber;
          });
      // check if we found the suggested most wise player was actually found
      if (mostWisePlayer != activePlayers_.end())
      {
        globalMostWisePlayer_.player = *mostWisePlayer;
        globalMostWisePlayer_.valid = true;
      }
      else
      {
        // fallback to own values if we did not find the suggested most wise player.
        Log<M_BRAIN>(LogLevel::WARNING)
            << "Suggested most wise player was not found. Fallback to own map...";
        globalMostWisePlayer_.player = &ownTeamPlayerData_;
        globalMostWisePlayer_.playerNumber = playerConfiguration_->playerNumber;
        globalMostWisePlayer_.valid = true;
      }
      break;
    }
  }

  searcherPosition_->localMostWisePlayerNumber = localMostWisePlayer_.playerNumber;
  searcherPosition_->globalMostWisePlayerNumber = globalMostWisePlayer_.playerNumber;
}

void SearcherPositionProvider::assignSearchAreas()
{
  // This method will fail if there is no searcher. Do not call the function in this case.
  assert(!searchers_.empty() && "Not able to assign search areas without any searcher");

  bool reassignmentRequired = false;

  // check if searchers are the same as last cycle
  if (searchers_.size() == searchAreas_.size())
  {
    for (auto& player : searchers_)
    {
      bool numMatch = false;
      // check if this player is assigned to any search area
      for (auto& searchArea : searchAreas_)
      {
        if (player->playerNumber == searchArea.assignedPlayerNumber)
        {
          numMatch = true;
          searchArea.setAssignedPlayer(player);
        }
      }
      // If a player has been replaced, reassignment of the search areas is needed.
      if (!numMatch)
      {
        reassignmentRequired = true;
        break;
      }
    }
  }
  else
  {
    rebuildSearchAreas();
    reassignmentRequired = true;
  }

  // The searchers have changed. The areas need to be reassigned.
  if (reassignmentRequired)
  {
    // Sets the positionToExplore for all areas.
    for (auto& area : searchAreas_)
    {
      float theoreticalBestCellValue = std::numeric_limits<float>::max();
      ProbCell const* theoreticalBestCell =
          &ballSearchMap_->cellFromPositionConst(area.defaultPosition);
      for (ProbCell const* cell : area.cells)
      {
        const float value = getValue(*cell);
        if (theoreticalBestCellValue < value)
        {
          theoreticalBestCell = cell;
          theoreticalBestCellValue = value;
        }
      }
      area.cellToExplore = theoreticalBestCell;
    }

    // Return the obvious assignment if there is only one searcher.
    if (searchers_.size() == 1)
    {
      searchAreas_[0].setAssignedPlayer(searchers_[0]);
      return;
    }

    // the consts for every searcher to get to the search area's cellToExplore
    Eigen::MatrixXi costs = Eigen::MatrixXi::Constant(searchers_.size(), searchers_.size(),
                                                      std::numeric_limits<int>::max());

    // calculate the costs for each searcher.
    for (unsigned int i = 0; i < searchAreas_.size(); i++)
    {
      for (unsigned int playerIndex = 0; playerIndex < searchers_.size(); playerIndex++)
      {
        const ProbCell* cellToExplore = searchAreas_[i].cellToExplore;
        costs(i, playerIndex) =
            static_cast<int>(timeToReachCell(*(searchers_[playerIndex]), *cellToExplore) * 1000.f);
      }
    }
    HungarianMethod minimizer;
    // minimize the overall costs to go to the cellToExplore for all searchers
    Eigen::Array2Xi minimumMatching = minimizer.findMaximumMatching(costs, true);
    // apply the minimizer's results.
    for (int col = 0; col < minimumMatching.cols(); col++)
    {
      auto playerIndex = static_cast<unsigned int>(minimumMatching(1, col));
      searchAreas_[minimumMatching(0, col)].setAssignedPlayer(searchers_[playerIndex]);
    }
  }
}

void SearcherPositionProvider::assignSearchPositions()
{
  // Find the best cellToExplore for each area.
  for (auto& area : searchAreas_)
  {
    // the currently targeted cell is being prioritized.
    float currentCosts = 0.9f * getCosts(*area.assignedPlayer, *area.cellToExplore);
    // search for a better target
    for (const auto& cell : area.cells)
    {
      const float costToAlternative = getCosts(*area.assignedPlayer, *cell);
      if (currentCosts > costToAlternative)
      {
        currentCosts = costToAlternative;
        area.cellToExplore = cell;
      }
    }
  }

  // Set suggested search positions for all robots that were assigned to an area..
  for (const auto& area : searchAreas_)
  {
    searcherPosition_->suggestedSearchPositions[area.assignedPlayerNumber - 1] =
        area.cellToExplore->position;
    ownTeamPlayerData_.suggestedSearchPositions[area.assignedPlayerNumber - 1] =
        area.cellToExplore->position;
    searcherPosition_->suggestedSearchPositionValid[area.assignedPlayerNumber - 1] = true;
    ownTeamPlayerData_.suggestedSearchPositionsValidity[area.assignedPlayerNumber - 1] = true;
  }
}

void SearcherPositionProvider::generateOwnSearchPose()
{
  // activePlayers are sorted by wisdom so the best player is at index 0
  // find the most wise player with valid data.
  if (globalMostWisePlayer_.player
          ->suggestedSearchPositionsValidity[playerConfiguration_->playerNumber - 1])
  {
    searcherPosition_->ownSearchPoseValid = true;
    searcherPosition_->searchPosition =
        globalMostWisePlayer_.player
            ->suggestedSearchPositions[playerConfiguration_->playerNumber - 1];
  }
  else
  {
    // Most wise player did not suggest a valid search position for us
    // Falling back to standing
    searcherPosition_->pose = robotPosition_->pose;
    searcherPosition_->ownSearchPoseValid = true;
    return;
  }

  // Generate a pose for the own robot for looking at the assigned search position.
  // This pose needs to be in a certain radius around the search pose (also not too close).
  const Vector2f position = robotPosition_->pose.position();
  const float xMin =
      -(fieldDimensions_->fieldLength / 2.f) + std::min(0.5f * maxBallDetectionRange_(), 1.2f);
  const float xMax = (-1.f) * xMin;
  const float yMin =
      -(fieldDimensions_->fieldWidth / 2.f) + std::min(0.5f * maxBallDetectionRange_(), 1.2f);
  const float yMax = (-1.f) * yMin;
  const Vector2f fakeRobotPosition = {Range<float>::clipToGivenRange(position.x(), xMin, xMax),
                                      Range<float>::clipToGivenRange(position.y(), yMin, yMax)};

  Vector2f relCellPosition = searcherPosition_->searchPosition - fakeRobotPosition;

  if (relCellPosition.norm() > minBallDetectionRange_())
  {
    const auto relativeCellAngle =
        static_cast<float>(atan2(relCellPosition.y(), relCellPosition.x()));
    if (relCellPosition.norm() > maxBallDetectionRange_() / 2.f)
    {
      // The robot is too far away from the search cell. Move towards it.
      relCellPosition = relCellPosition.normalized() *
                        (relCellPosition.norm() - (maxBallDetectionRange_() / 2.f));
    }
    else
    {
      // The robot is close enough to the search cell. Keep current distance.
      relCellPosition = Vector2f::Zero();
    }
    searcherPosition_->pose = Pose(fakeRobotPosition + relCellPosition, relativeCellAngle);
  }
  else
  {
    // the robot is too close to the target position. Move away.
    searcherPosition_->pose =
        Pose(robotPosition_->pose.x() + relCellPosition.x() - (minBallDetectionRange_() * 1.5f),
             robotPosition_->pose.y() + relCellPosition.y());
  }
}

float SearcherPositionProvider::timeToReachPosition(const TeamPlayer& player,
                                                    const Vector2f& position) const
{
  const Vector2f relPosition = position - player.pose.position();
  // TODO: Is 15cm per second a good approximation?
  const float walkTimeDistance = relPosition.norm() / 0.18f;
  // TODO: Is 10s per 180° a good approximation?
  const float cellOrientation = std::atan2(relPosition.y(), relPosition.x());
  const auto rotateTimeDistance =
      static_cast<float>(Angle::angleDiff(cellOrientation, player.pose.angle()) * 10.f / M_PI);
  // TODO: Is 10s a good approximation?
  const float fallenPenalty = player.fallen ? 10.0f : 0.0f;

  return walkTimeDistance + rotateTimeDistance + fallenPenalty;
}

float SearcherPositionProvider::timeToReachCell(const TeamPlayer& player,
                                                const ProbCell& cell) const
{
  return timeToReachPosition(player, cell.position);
}

float SearcherPositionProvider::getValue(const ProbCell& cell) const
{
  return cell.probability * probabilityWeight_() +
         std::min(maxAgeValueContribution_(), static_cast<float>(cell.age));
}

void SearcherPositionProvider::sendDebug()
{
  debug().update(mount_ + ".searcherCount", searchers_.size());
  if (!searchers_.empty())
  {
    if (debug().isSubscribed(mount_ + ".voronoiSeeds"))
    {
      VecVector2f seeds;
      for (const auto& seed : voronoiSeeds_()[searchers_.size() - 1])
      {
        seeds.emplace_back(seed.x() * fieldLength_ * 0.5f, seed.y() * fieldWidth_ * 0.5f);
      }
      debug().update(mount_ + ".voronoiSeeds", seeds);
    }
  }
}

float SearcherPositionProvider::getCosts(const TeamPlayer& player, const ProbCell& cellToExplore)
{
  return (timeToReachCell(player, cellToExplore) + 2.f) / getValue(cellToExplore);
}

void SearcherPositionProvider::rebuildSearchAreas()
{
  searchAreas_.clear();
  searchAreas_.reserve(searchers_.size());

  if (searchers_.empty())
  {
    return;
  }
  const std::vector<Vector2f> seeds = gameControllerState_->setPlay == SetPlay::CORNER_KICK
                                          ? cornerKickVoronoiSeeds_()[searchers_.size() - 1]
                                          : voronoiSeeds_()[searchers_.size() - 1];

  for (const auto& seed : seeds)
  {
    SearchArea area;
    area.voronoiSeed = {seed.x() * fieldLength_ / 2.f, seed.y() * fieldWidth_ / 2.f};
    area.defaultPosition = area.voronoiSeed;
    area.cellToExplore = &ballSearchMap_->cellFromPositionConst(area.defaultPosition);
    searchAreas_.emplace_back(area);
  }

  // voronoi (https://en.wikipedia.org/wiki/Voronoi_diagram)
  // The field is divided into so called searchAreas. Division is done by reading
  // the seeds (aka generators) coming from the config and do some voronoi on them.
  for (const auto& cell : ballSearchMap_->probabilityList)
  {
    SearchArea* minimumDistanceArea = &(searchAreas_[0]);
    float minimumDistance = std::numeric_limits<float>::max();
    int areaNum = 0;

    for (auto& area : searchAreas_)
    {
      Vector2f relDistance = area.voronoiSeed - cell->position;
      float areaDistance = relDistance.squaredNorm();
      if (areaDistance < minimumDistance)
      {
        minimumDistance = areaDistance;
        minimumDistanceArea = &area;
      }
      areaNum++;
    }
    minimumDistanceArea->cells.emplace_back(cell);
  }
}
