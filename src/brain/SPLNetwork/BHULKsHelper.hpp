#pragma once

#include "Data/PlayingRoles.hpp"

#include "Definitions/BHULKsStandardMessage.h"


namespace B_HULKs
{
  /**
   * @brief bhulkToPlayingRole converts a BHULKs role to a playing role
   * @param role the role as BHULKs role
   * @return the role as playing role
   */
  inline PlayingRole bhulkToPlayingRole(const Role role)
  {
    switch (role)
    {
      case Role::King:
        return PlayingRole::KEEPER;
      case Role::Queen:
        return PlayingRole::STRIKER;
      case Role::Rook:
        return PlayingRole::DEFENDER;
      case Role::Knight:
        return PlayingRole::SUPPORT_STRIKER;
      case Role::Bishop:
        return PlayingRole::BISHOP;
      case Role::beatenPieces:
      default:
        return PlayingRole::NONE;
    }
  }
  /**
   * @brief playingToBHULKRole converts a playing role to a BHULKs role
   * @param role the role as playing role
   * @return the role as BHULKs role
   */
  inline Role playingToBHULKRole(const PlayingRole role)
  {
    switch (role)
    {
      case PlayingRole::KEEPER:
        return Role::King;
      case PlayingRole::STRIKER:
        return Role::Queen;
      case PlayingRole::DEFENDER:
        return Role::Rook;
      case PlayingRole::SUPPORT_STRIKER:
        return Role::Knight;
      case PlayingRole::BISHOP:
        return Role::Bishop;
      case PlayingRole::NONE:
      default:
        return Role::beatenPieces;
    }
  }
}
