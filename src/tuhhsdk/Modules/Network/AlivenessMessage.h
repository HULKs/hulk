#ifndef ALIVENESSMESSAGE_H
#define ALIVENESSMESSAGE_H

#include <string>
#include <cstdio>
#include <string.h>

/**
 * This message is broadcasted by the NAO, to signal
 * its presence e.g. to an external Software tool.
 *
 * @author Robert Oehlmann
 * @author Finn Poppinga
 */
struct AlivenessMessage {
  AlivenessMessage(const std::string& bname, const std::string& hname){
    strncpy(header, "LIVE", sizeof header);
    sprintf(bodyname, "%.31s", bname.c_str());
    sprintf(headname, "%.31s", hname.c_str());
  }

  char header[4];   //"LIVE"
  char bodyname[32];
  char headname[32];
};

#endif // ALIVENESSMESSAGE_H
