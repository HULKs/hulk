// fix boost avio and LogLevel problems
#ifdef ERROR
#undef ERROR
#endif

// of course windows also defines a makro named DM_UPDATE :-(
#ifdef DM_UPDATE
#undef DM_UPDATE
#endif
