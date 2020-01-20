#ifndef ALIVENESSMESSAGE_H
#define ALIVENESSMESSAGE_H

#include <cstdio>
#include <string.h>
#include <string>

#include "Modules/Configuration/Configuration.h"

/**
 * This message is broadcasted by the NAO, to signal
 * its presence e.g. to an external Software tool.
 *
 * @author Robert Oehlmann
 * @author Finn Poppinga
 */
struct AlivenessMessage
{
  AlivenessMessage(const std::string& bname, const std::string& hname, Configuration& config)
  {
    sprintf(bodyname, "%.31s", bname.c_str());
    sprintf(headname, "%.31s", hname.c_str());
    playerNum = static_cast<uint8_t>(config.get("Brain.Config", "general.playerNumber").asInt32());
  }

  char header[4] = {'L', 'I', 'V', 'E'};
  char bodyname[32];
  char headname[32];
  uint8_t playerNum;
};

#endif // ALIVENESSMESSAGE_H
