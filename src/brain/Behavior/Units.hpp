#pragma once

#include "Modules/NaoProvider.h"

#include "ActionCommand.hpp"
#include "DataSet.hpp"

#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-parameter"
#elif defined __GNUC__
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wunused-parameter"
#endif

#include "BehaviorUnits/Head/CameraCalibration.hpp"
#include "BehaviorUnits/Head/LookAround.hpp"
#include "BehaviorUnits/Head/LookForward.hpp"
#include "BehaviorUnits/Head/TrackBall.hpp"
#include "BehaviorUnits/Head/ActiveVision.hpp"
#include "BehaviorUnits/Skills/WalkToPose.hpp"
#include "BehaviorUnits/Skills/WalkBehindBall.hpp"
#include "BehaviorUnits/Skills/Dribble.hpp"
#include "BehaviorUnits/Skills/Kick.hpp"
#include "BehaviorUnits/Skills/InWalkKick.hpp"
#include "BehaviorUnits/Skills/Rotate.hpp"
#include "BehaviorUnits/Skills/StandUp.hpp"
#include "BehaviorUnits/Skills/SearchForBall.hpp"
#include "BehaviorUnits/Roles/Bishop.hpp"
#include "BehaviorUnits/Roles/Defender.hpp"
#include "BehaviorUnits/Roles/Demo.hpp"
#include "BehaviorUnits/Roles/Keeper.hpp"
#include "BehaviorUnits/Roles/ReplacementKeeper.hpp"
#include "BehaviorUnits/Roles/ShootOnHeadTouch.hpp"
#include "BehaviorUnits/Roles/Striker.hpp"
#include "BehaviorUnits/Roles/SupportStriker.hpp"
#include "BehaviorUnits/GameStates/Initial.hpp"
#include "BehaviorUnits/GameStates/Ready.hpp"
#include "BehaviorUnits/GameStates/Set.hpp"
#include "BehaviorUnits/GameStates/PenaltyShootout.hpp"
#include "BehaviorUnits/GameStates/Playing.hpp"
#include "BehaviorUnits/GameStates/Finished.hpp"
#include "BehaviorUnits/NotPenalized.hpp"
#include "BehaviorUnits/RootBehavior.hpp"

#ifdef __clang__
#pragma clang diagnostic pop
#elif defined __GNUC__
#pragma GCC diagnostic pop
#endif
