/**
 * @file DcmConverter.hpp
 * @brief File providing the sensor and actuator communication.
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 *
 * Two classes are introduces in order to communicate with the AL framework.
 */

#ifndef DCMCONVERTER_HPP
#define DCMCONVERTER_HPP

#include <string>
#include <vector>

#include <alvalue/alvalue.h>


/**
 * @class DcmConverter
 * @brief Converter for Commands
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 *
 * Convertation from command vectors to ALValue commands.
 */
class DcmConverter
{
public:

    /**
     * @brief Converts an alias in vector form to an ALValue alias
     * @param alias The alias in std::string vector form
     * @return The alias in ALValue form
     */
    static AL::ALValue& convertAlias( const std::vector<std::string>& alias)
    {
        static AL::ALValue aliasArray;
        aliasArray.arraySetSize(2);
        aliasArray[0] = alias.at(0);
        aliasArray[1].arraySetSize((int) alias.size() -1);

        for (int i = 0; i < (int) alias.size()-1; i++)
            aliasArray[1][i] = alias.at(i+1);

        return aliasArray;
    }

};
#endif // DCMCONVERTER_HPP
