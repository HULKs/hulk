#pragma once

#include "Data/PlayingRoles.hpp"
#include "Messages/DevilSmashStandardMessage.hpp"


namespace DevilSmash
{
  /**
   * @brief bhulkToPlayingRole converts a DevilSMASH role to a playing role
   * @param role the role as DevilSMASH role
   * @return the role as playing role
   */
  inline PlayingRole dsRoleToPlayingRole(const Role role)
  {
    switch (role)
    {
      case Role::NONE:
        return PlayingRole::NONE;
      case Role::KEEPER:
        return PlayingRole::KEEPER;
      case Role::REPLACEMENT_KEEPER:
        return PlayingRole::REPLACEMENT_KEEPER;
      case Role::DEFENDER:
        return PlayingRole::DEFENDER;
      case Role::PUNISHER:
        return PlayingRole::BISHOP;
      case Role::SUPPORT:
        return PlayingRole::SUPPORT_STRIKER;
      case Role::STRIKER:
        return PlayingRole::STRIKER;
      case Role::LOSER:
        return PlayingRole::LOSER;
      case Role::SEARCHER:
        return PlayingRole::SEARCHER;
      default:
        return PlayingRole::NONE;
    }
  }
  /**
   * @brief playingToDSRole converts a playing role to a DevilSMASH role
   * @param role the role as playing role
   * @return the role as DevilSMASH role
   */
  inline Role playingToDSRole(const PlayingRole role)
  {
    switch (role)
    {
      case PlayingRole::NONE:
        return Role::NONE;
      case PlayingRole::KEEPER:
        return Role::KEEPER;
      case PlayingRole::REPLACEMENT_KEEPER:
        return Role::REPLACEMENT_KEEPER;
      case PlayingRole::DEFENDER:
        return Role::DEFENDER;
      case PlayingRole::BISHOP:
        return Role::PUNISHER;
      case PlayingRole::SUPPORT_STRIKER:
        return Role::SUPPORT;
      case PlayingRole::STRIKER:
        return Role::STRIKER;
      case PlayingRole::LOSER:
        return Role::LOSER;
      case PlayingRole::SEARCHER:
        return Role::SEARCHER;
      default:
        return Role::MAX;
    }
  }
} // namespace DevilSmash
