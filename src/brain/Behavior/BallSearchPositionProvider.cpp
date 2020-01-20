#include <Eigen/Dense>

#include "Tools/Chronometer.hpp"
#include "Tools/Math/HungarianMethod.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Math/Range.hpp"

#include "print.h"

#include "BallSearchPositionProvider.hpp"


BallSearchPositionProvider::BallSearchPositionProvider(const ModuleManagerInterface& manager)
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
  , keeperReach_(*this, "keeperReach", [] {})
  , replacementKeeperAdvantage_(*this, "replacementKeeperAdvantage", [] {})
  , isOnePlayerReplacementKeeper_(*this, "isOnePlayerReplacementKeeper", [] {})
  , searchPosition_(*this)
  , fieldLength_(fieldDimensions_->fieldLength)
  , fieldWidth_(fieldDimensions_->fieldWidth)
{
  rebuildSearchAreas();
}


void BallSearchPositionProvider::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time");

    // Updating the map while not playing may make things worse.
    if (gameControllerState_->gameState != GameState::PLAYING)
    {
      // While not playing the search pose is set to a position near the center of the field.
      searchPosition_->pose = Pose(-0.5f, 0.f);
      searchPosition_->searchPosition = {-0.5f, 0.f};
      for (auto& suggestion : searchPosition_->suggestedSearchPositions)
      {
        suggestion = {-0.5f, 0.f};
      }
    }
    else
    {
      activePlayers_.clear();
      explorers_.clear();

      generateOwnTeamPlayerData();
      searchPosition_->availableForSearch = ownTeamPlayerData_.isAvailableForBallSearch;
      keeperIsInActivePlayers_ = false;
      keeperIsNearGoal_ = false;
      keeperPosition_ =
          Vector2f(fieldDimensions_->fieldLength * 0.5f, fieldDimensions_->fieldWidth * 0.5f);

      // Add myself to active players
      if (!ownTeamPlayerData_.penalized)
      {
        activePlayers_.push_back(&ownTeamPlayerData_);
        // check whether keeper is in active players and get keeper position
        if (ownTeamPlayerData_.playerNumber == 1)
        {
          keeperIsInActivePlayers_ = true;
          keeperPosition_ = ownTeamPlayerData_.pose.position;
        }
      }

      // Add all other team players to active players.
      for (auto& teamPlayer : teamPlayers_->players)
      {
        if (teamPlayer.penalized)
        {
          continue;
        }
        activePlayers_.push_back(&teamPlayer);

        // check whether keeper is in active players and get keeper position
        if (teamPlayer.playerNumber == 1)
        {
          keeperIsInActivePlayers_ = true;
          keeperPosition_ = teamPlayer.pose.position;
        }
      }

      if (activePlayers_.empty())
      {
        return;
      }

      // Check whether keeper is near goal
      if ((keeperPosition_ - goalPosition_).norm() < keeperReach_())
      {
        keeperIsNearGoal_ = true;
      }

      // Sort player by wisdom to find the player with the oldest, continuously updated map.
      std::sort(activePlayers_.begin(), activePlayers_.end(),
                [](const TeamPlayer* a, const TeamPlayer* b) {
                  return (a->timestampBallSearchMapUnreliable == b->timestampBallSearchMapUnreliable
                              ? a->playerNumber < b->playerNumber
                              : a->timestampBallSearchMapUnreliable <
                                    b->timestampBallSearchMapUnreliable);
                });

      // Check whether replacement keeper is necessary;
      // if only one player that is keeper is in active players, replacement keeper is not
      // necessary;
      // if only one player that is not keeper is in active players, replacement keeper
      // will only be assigned if parameter "isOnePlayerReplacementKeeper" is true;
      // in all other cases a replacement keeper is necessary;
      // then find the best replacement keeper
      if ((activePlayers_.size() == 1 &&
           (keeperIsInActivePlayers_ || !isOnePlayerReplacementKeeper_())) ||
          keeperIsNearGoal_)
      {
        searchPosition_->replacementKeeperNumber = 0;
      }
      else
      {
        float distanceRobotToKeeperPos =
            ((Vector2f(fieldDimensions_->fieldLength * 0.5f, fieldDimensions_->fieldWidth * 0.5f)) -
             goalPosition_)
                .norm();

        for (auto& teamPlayer : activePlayers_)
        {
          // Assign replacement keeper to robot closest to goal except keeper
          if (((goalPosition_ - Vector2f(teamPlayer->pose.position)).norm() <
               distanceRobotToKeeperPos) &&
              (teamPlayer->playerNumber != 1))
          {
            searchPosition_->replacementKeeperNumber = teamPlayer->playerNumber;
            distanceRobotToKeeperPos = (goalPosition_ - (teamPlayer->pose.position)).norm();
          }
        }
        assert(searchPosition_->replacementKeeperNumber != 0);

        // if keeper is almost as close to goal as replacement keeper, sending replacement keeper to
        // goal is unnecessary
        if (((keeperPosition_ - goalPosition_).norm() - distanceRobotToKeeperPos) <
            replacementKeeperAdvantage_())
        {
          searchPosition_->replacementKeeperNumber = 0;
        }
      }

      // Find all active players that report themselves as available for search and add them to the
      // explorers list
      for (auto& teamPlayer : activePlayers_)
      {
        // Check if the robot is ready to search for the ball (he may exclude himself)
        if (teamPlayer->playerNumber != searchPosition_->replacementKeeperNumber &&
            teamPlayer->playerNumber != 1 && teamPlayer->isHULK)
        {
          explorers_.push_back(teamPlayer);
        }
      }

      calculateMostWisePlayer();
      assert(localMostWisePlayer_.valid && "localMostWisePlayer needs to be valid");
      assert(globalMostWisePlayer_.valid && "globalMostWisePlayer needs to be valid");
      ownTeamPlayerData_.mostWisePlayerNumber = localMostWisePlayer_.playerNumber;

      if (searchPosition_->replacementKeeperNumber != 0)
      {
        assignReplacementKeeperPosition();
      }

      if (gameControllerState_->setPlay == SetPlay::CORNER_KICK && !searchAreasCleared_)
      {
        searchAreas_.clear();
        searchAreasCleared_ = true;
      }

      if (gameControllerState_->setPlay != SetPlay::CORNER_KICK && searchAreasCleared_)
      {
        searchAreasCleared_ = false;
      }


      if (!explorers_.empty())
      {
        assignSearchAreas();
        assignSearchPositions();
      }

      // Do not calculate a search pose for a robot that is not available for
      // search.
      if (ownTeamPlayerData_.isAvailableForBallSearch)
      {
        generateOwnSearchPose();
      }
    }
  }
  sendDebug();
}

void BallSearchPositionProvider::generateOwnTeamPlayerData()
{
  // create a teamPlayer object for ourself.
  ownTeamPlayerData_.playerNumber = playerConfiguration_->playerNumber;
  ownTeamPlayerData_.isHULK = true;
  ownTeamPlayerData_.pose = robotPosition_->pose;
  ownTeamPlayerData_.ballPosition = ballState_->position;
  ownTeamPlayerData_.timeWhenBallWasSeen = ballState_->timeWhenLastSeen;
  ownTeamPlayerData_.fallen = bodyPose_->fallen;
  ownTeamPlayerData_.penalized = gameControllerState_->penalty != Penalty::NONE;
  ownTeamPlayerData_.headYaw = jointSensorData_->angles[keys::joints::HEAD_YAW];
  ownTeamPlayerData_.currentSearchPosition = finalSearchPose_.position;
  ownTeamPlayerData_.isAvailableForBallSearch =
      gameControllerState_->penalty == Penalty::NONE && playerConfiguration_->playerNumber != 1;
}

void BallSearchPositionProvider::calculateMostWisePlayer()
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
    using systemTime_t =
        decltype(localMostWisePlayer_.player->timestampBallSearchMapUnreliable.getSystemTime());
    if (Hysteresis<systemTime_t>::smallerThan(
            activePlayers_[0]->timestampBallSearchMapUnreliable.getSystemTime(),
            localMostWisePlayer_.player->timestampBallSearchMapUnreliable.getSystemTime(), 5000,
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
      for (auto& teamPlayer : activePlayers_)
      {
        if (globalMostWisePlayer_.playerNumber == teamPlayer->playerNumber)
        {
          // we found the suggested most wise player.
          globalMostWisePlayer_.player = teamPlayer;
          globalMostWisePlayer_.valid = true;
          break;
        }
      }
      // check if we found the suggested most wise player was actually found
      if (!globalMostWisePlayer_.valid)
      {
        // fallback to own values if we did not find the suggested most wise player.
        Log(LogLevel::WARNING)
            << "Suggested most wise player was not found! Fallback to own map...";
        globalMostWisePlayer_.player = &ownTeamPlayerData_;
        globalMostWisePlayer_.playerNumber = playerConfiguration_->playerNumber;
        globalMostWisePlayer_.valid = true;
      }
      break;
    }
  }

  searchPosition_->localMostWisePlayerNumber = localMostWisePlayer_.playerNumber;
  searchPosition_->globalMostWisePlayerNumber = globalMostWisePlayer_.playerNumber;
}

void BallSearchPositionProvider::assignSearchAreas()
{
  // This method will fail if there is no explorer. Do not call the function in this case.
  assert(!explorers_.empty() && "Not able to assign search areas without any explorer");

  bool reassignmentRequired = false;

  // check if explorers are the same as last cycle
  if (explorers_.size() == searchAreas_.size())
  {
    for (auto& player : explorers_)
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

  // The explorers have changed. The areas need to be reassigned.
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

    // Return the obvious assignment if there is only one explorer.
    if (explorers_.size() == 1)
    {
      searchAreas_[0].setAssignedPlayer(explorers_[0]);
      return;
    }

    // the consts for every explorer to get to the search area's cellToExplore
    Eigen::MatrixXi costs = Eigen::MatrixXi::Constant(explorers_.size(), explorers_.size(),
                                                      std::numeric_limits<int>::max());

    // calculate the costs for each explorer.
    for (unsigned int i = 0; i < searchAreas_.size(); i++)
    {
      for (unsigned int playerIndex = 0; playerIndex < explorers_.size(); playerIndex++)
      {
        const ProbCell* cellToExplore = searchAreas_[i].cellToExplore;
        costs(i, playerIndex) =
            static_cast<int>(timeToReachCell(*(explorers_[playerIndex]), *cellToExplore) * 1000.f);
      }
    }
    HungarianMethod minimizer;
    // minimize the overall costs to go to the cellToExplore for all explorers
    Eigen::Array2Xi minimumMatching = minimizer.findMaximumMatching(costs, true);
    // apply the minimizer's results.
    for (int col = 0; col < minimumMatching.cols(); col++)
    {
      auto playerIndex = static_cast<unsigned int>(minimumMatching(1, col));
      searchAreas_[minimumMatching(0, col)].setAssignedPlayer(explorers_[playerIndex]);
    }
  }
}

void BallSearchPositionProvider::assignSearchPositions()
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
    searchPosition_->suggestedSearchPositions[area.assignedPlayerNumber - 1] =
        area.cellToExplore->position;
    ownTeamPlayerData_.suggestedSearchPositions[area.assignedPlayerNumber - 1] =
        area.cellToExplore->position;
    searchPosition_->suggestedSearchPositionValid[area.assignedPlayerNumber - 1] = true;
    ownTeamPlayerData_.suggestedSearchPositionsValidity[area.assignedPlayerNumber - 1] = true;
  }
}

void BallSearchPositionProvider::assignReplacementKeeperPosition()
{
  searchPosition_->replacementKeeperPose = goalPosition_;
  searchPosition_->suggestedSearchPositions[searchPosition_->replacementKeeperNumber - 1] =
      goalPosition_;
  ownTeamPlayerData_.suggestedSearchPositions[searchPosition_->replacementKeeperNumber - 1] =
      goalPosition_;
  searchPosition_->suggestedSearchPositionValid[searchPosition_->replacementKeeperNumber - 1] =
      true;
  ownTeamPlayerData_
      .suggestedSearchPositionsValidity[searchPosition_->replacementKeeperNumber - 1] = true;
}

void BallSearchPositionProvider::generateOwnSearchPose()
{
  if (playerConfiguration_->playerNumber == searchPosition_->replacementKeeperNumber)
  {
    searchPosition_->pose = searchPosition_->replacementKeeperPose;
    searchPosition_->ownSearchPoseValid = true;
    searchPosition_->searchPosition = searchPosition_->replacementKeeperPose.position;
    return;
  }

  // activePlayers are sorted by wisdom so the best player is at index 0
  // find the most wise player with valid data.
  if (globalMostWisePlayer_.player
          ->suggestedSearchPositionsValidity[playerConfiguration_->playerNumber - 1])
  {
    searchPosition_->ownSearchPoseValid = true;
    searchPosition_->searchPosition =
        globalMostWisePlayer_.player
            ->suggestedSearchPositions[playerConfiguration_->playerNumber - 1];
  }
  else
  {
    Log(LogLevel::WARNING) << "Most wise player did not suggest a valid searchPosition for me. "
                              "Falling back to another player";
    for (const auto* player : activePlayers_)
    {
      if (player->suggestedSearchPositionsValidity[playerConfiguration_->playerNumber - 1])
      {
        searchPosition_->ownSearchPoseValid = true;
        searchPosition_->searchPosition =
            player->suggestedSearchPositions[playerConfiguration_->playerNumber - 1];
        break;
      }
    }
    if (!searchPosition_->ownSearchPoseValid)
    {
      Log(LogLevel::ERROR)
          << "No teamPlayer suggested any valid searchPosition for me. Falling back to {0.f, 0.f}";
      searchPosition_->ownSearchPoseValid = true;
      searchPosition_->searchPosition = {0.f, 0.f};
    }
  }

  // Generate a pose for the own robot for looking at the assigned search position.
  // This pose needs to be in a certain radius around the search pose (also not too close).
  const Vector2f position = robotPosition_->pose.position;
  const float xMin =
      -(fieldDimensions_->fieldLength / 2.f) + std::min(0.5f * maxBallDetectionRange_(), 1.2f);
  const float xMax = (-1.f) * xMin;
  const float yMin =
      -(fieldDimensions_->fieldWidth / 2.f) + std::min(0.5f * maxBallDetectionRange_(), 1.2f);
  const float yMax = (-1.f) * yMin;
  const Vector2f fakeRobotPosition = {Range<float>::clipToGivenRange(position.x(), xMin, xMax),
                                      Range<float>::clipToGivenRange(position.y(), yMin, yMax)};

  Vector2f relCellPosition = searchPosition_->searchPosition - fakeRobotPosition;

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
    searchPosition_->pose = Pose(fakeRobotPosition + relCellPosition, relativeCellAngle);
  }
  else
  {
    // the robot is too close to the target position. Move away.
    searchPosition_->pose = Pose(robotPosition_->pose.position.x() + relCellPosition.x() -
                                     (minBallDetectionRange_() * 1.5f),
                                 robotPosition_->pose.position.y() + relCellPosition.y());
  }
}

float BallSearchPositionProvider::timeToReachPosition(const TeamPlayer& player,
                                                      const Vector2f position) const
{
  const Vector2f relPosition = position - player.pose.position;
  // TODO: Is 15cm per second a good approximation?
  const float walkTimeDistance = relPosition.norm() / 0.18f;
  // TODO: Is 10s per 180° a good approximation?
  const float cellOrientation = std::atan2(relPosition.y(), relPosition.x());
  const auto rotateTimeDistance =
      static_cast<float>(Angle::angleDiff(cellOrientation, player.pose.orientation) * 10.f / M_PI);
  // TODO: Is 10s a good approximation?
  const float fallenPenalty = player.fallen ? 10.0f : 0.0f;

  return walkTimeDistance + rotateTimeDistance + fallenPenalty;
}

float BallSearchPositionProvider::timeToReachCell(const TeamPlayer& player,
                                                  const ProbCell& cell) const
{
  return timeToReachPosition(player, cell.position);
}

float BallSearchPositionProvider::getValue(const ProbCell& cell) const
{
  return cell.probability * probabilityWeight_() +
         std::min(maxAgeValueContribution_(), static_cast<float>(cell.age));
}

void BallSearchPositionProvider::sendDebug()
{
  debug().update(mount_ + ".explorerCount", explorers_.size());
  if (!explorers_.empty())
  {
    if (debug().isSubscribed(mount_ + ".voronoiSeeds"))
    {
      VecVector2f seeds;
      for (auto& seed : voronoiSeeds_()[explorers_.size() - 1])
      {
        seeds.emplace_back(seed.x() * fieldLength_ * 0.5f, seed.y() * fieldWidth_ * 0.5f);
      }
      debug().update(mount_ + ".voronoiSeeds", seeds);
    }
  }
}

float BallSearchPositionProvider::getCosts(const TeamPlayer& player, const ProbCell& cellToExplore)
{
  return (timeToReachCell(player, cellToExplore) + 2.f) / getValue(cellToExplore);
}

void BallSearchPositionProvider::rebuildSearchAreas()
{
  searchAreas_.clear();
  searchAreas_.reserve(explorers_.size());

  if (explorers_.empty())
  {
    return;
  }
  std::vector<Vector2f> Seeds = gameControllerState_->setPlay == SetPlay::CORNER_KICK
                                    ? cornerKickVoronoiSeeds_()[explorers_.size() - 1]
                                    : voronoiSeeds_()[explorers_.size() - 1];

  for (auto& seed : Seeds)
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
  for (auto& cell : ballSearchMap_->probabilityList_)
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
    minimumDistanceArea->cells.push_back(cell);
  }
}
