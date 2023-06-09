// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#include "osstd.hxx"

#include "errdata.hxx"


//#include "_tracetagimpl.h"

BOOL            g_fDisableTracingForced     = fFalse;
BOOL            g_fTracing                  = fFalse;
TRACEINFO       g_rgtraceinfo[ JET_tracetagMax ];
DWORD           g_tidTrace                  = 0;

const WCHAR * const g_rgwszTraceDesc[] =
{
    L"ESE_TRACETAG_SZ_ARRAY"
};

C_ASSERT( JET_tracetagMax == _countof(g_rgwszTraceDesc) );


#include < stdio.h >



class COSInfoString
{
    public:

        COSInfoString( const size_t cch )
        {
            memset( m_szInfo + cch, 0, sizeof( COSInfoString ) - offsetof( COSInfoString, m_szInfo ) );
        }

        ~COSInfoString() {}

        char* String() { return m_szInfo; }

    public:

        static SIZE_T OffsetOfILE() { return offsetof( COSInfoString, m_ile ); }

    private:

        CInvasiveList< COSInfoString, OffsetOfILE >::CElement   m_ile;
        char                                                    m_szInfo[ 1 ];
};

class COSInfoStringW
{
    public:

        COSInfoStringW( const size_t cch )
        {
            memset( m_wszInfo + cch, 0, sizeof( COSInfoStringW ) - offsetof( COSInfoStringW, m_wszInfo ) );
        }

        ~COSInfoStringW() {}

        WCHAR* String() { return m_wszInfo; }

    public:

        static SIZE_T OffsetOfILE() { return offsetof( COSInfoStringW, m_ile ); }

    private:

        CInvasiveList< COSInfoStringW, OffsetOfILE >::CElement  m_ile;
        WCHAR                                                   m_wszInfo[ 1 ];
};

class COSThreadContext
{
    public:

        COSThreadContext();
        ~COSThreadContext();

        char* AllocInfoString( const size_t cch );
        WCHAR* AllocInfoStringW( const size_t cch );
        void FreeInfoStrings();

        void Indent( const INT dLevel );
        INT Indent();

        void SuspendGC();
        void ResumeGC();

    private:

        friend class COSThreadInfo;

        volatile ULONG   m_cref;

    private:

        CInvasiveList< COSInfoString, COSInfoString::OffsetOfILE >      m_listInfoString;
        CInvasiveList< COSInfoStringW, COSInfoStringW::OffsetOfILE >    m_listInfoStringW;
        INT                                                             m_cIndent;
        INT                                                             m_cSuspendGC;
};

inline COSThreadContext::COSThreadContext()
    :   m_cref( 0 ),
        m_cIndent( 0 ),
        m_cSuspendGC( 0 )
{
}

inline COSThreadContext::~COSThreadContext()
{
    FreeInfoStrings();
}

inline char* COSThreadContext::AllocInfoString( const size_t cch )
{
    const size_t            cbAlloc     = sizeof( COSInfoString ) + cch * sizeof( char );
    COSInfoString*          pinfostr    = NULL;
    if ( cbAlloc >= sizeof( COSInfoString ) && cbAlloc >= ( cch * sizeof( char ) ) )
    {
        pinfostr = reinterpret_cast< COSInfoString* >( LocalAlloc( 0, cbAlloc ) );
    }
    else{
        AssertSz( FALSE, "cbAlloc Overflow." );
    }
    if ( pinfostr )
    {
        new( pinfostr ) COSInfoString( cch );
        m_listInfoString.InsertAsNextMost( pinfostr );
    }

    return ( NULL != pinfostr ? pinfostr->String() : NULL );
}

inline WCHAR* COSThreadContext::AllocInfoStringW( const size_t cch )
{
    const size_t            cbAlloc     = sizeof( COSInfoStringW ) + cch * sizeof( WCHAR );
    COSInfoStringW*         pinfostr    = NULL;
    if ( cbAlloc >= sizeof( COSInfoStringW ) && cbAlloc >= ( cch * sizeof( WCHAR ) ) )
    {
        pinfostr = reinterpret_cast< COSInfoStringW* >( LocalAlloc( 0, cbAlloc ) );
    }
    else{
        AssertSz( FALSE, "cbAlloc Overflow." );
    }
    if ( pinfostr )
    {
        new( pinfostr ) COSInfoStringW( cch );
        m_listInfoStringW.InsertAsNextMost( pinfostr );
    }

    return ( NULL != pinfostr ? pinfostr->String() : NULL );
}

inline void COSThreadContext::FreeInfoStrings()
{
    while ( m_listInfoString.PrevMost() )
    {
        COSInfoString* const pinfostr = m_listInfoString.PrevMost();
        m_listInfoString.Remove( pinfostr );

        pinfostr->~COSInfoString();
        LocalFree( pinfostr );
    }
    while ( m_listInfoStringW.PrevMost() )
    {
        COSInfoStringW* const pinfostr = m_listInfoStringW.PrevMost();
        m_listInfoStringW.Remove( pinfostr );

        pinfostr->~COSInfoStringW();
        LocalFree( pinfostr );
    }
}

inline void COSThreadContext::Indent( const INT dLevel )
{
    m_cIndent = dLevel ? m_cIndent + dLevel : 0;
}

inline INT COSThreadContext::Indent()
{
    return m_cIndent;
}

inline void COSThreadContext::SuspendGC()
{
    m_cSuspendGC++;
}

inline void COSThreadContext::ResumeGC()
{
    m_cSuspendGC--;

    if ( !m_cSuspendGC )
    {
        FreeInfoStrings();
    }
}


class COSThreadInfo
{
    public:

        COSThreadInfo()
        {
            m_tid   = DWORD( ~0 );
            m_ptc   = NULL;
        }

        COSThreadInfo(  const DWORD&            tid,
                        COSThreadContext* const ptc )
        {
            m_tid   = tid;
            m_ptc   = ptc;

            if ( m_ptc )
            {
                AtomicIncrement( (LONG*)&m_ptc->m_cref );
            }
        }

        ~COSThreadInfo()
        {
            if ( m_ptc && !( AtomicDecrement( (LONG*)&m_ptc->m_cref ) ) )
            {
                m_ptc->~COSThreadContext();
                LocalFree( m_ptc );
            }

            m_tid   = DWORD( ~0 );
            m_ptc   = NULL;
        }

        COSThreadInfo& operator=( const COSThreadInfo& threadinfo )
        {
            if ( threadinfo.m_ptc )
            {
                AtomicIncrement( (LONG*)&threadinfo.m_ptc->m_cref );
            }

            if ( m_ptc && !( AtomicDecrement( (LONG*)&m_ptc->m_cref ) ) )
            {
                m_ptc->~COSThreadContext();
                LocalFree( m_ptc );
            }

            m_tid   = threadinfo.m_tid;
            m_ptc   = threadinfo.m_ptc;

            return *this;
        }

    public:

        DWORD               m_tid;
        COSThreadContext*   m_ptc;
};

typedef CTable< DWORD, COSThreadInfo > COSThreadTable;

inline INT COSThreadTable::CKeyEntry:: Cmp( const DWORD& tid ) const
{
    return m_tid - tid;
}

inline INT COSThreadTable::CKeyEntry:: Cmp( const COSThreadTable::CKeyEntry& keyentry ) const
{
    return Cmp( keyentry.m_tid );
}

CRITICAL_SECTION    g_csThreadTable;
BOOL                g_fcsThreadTableInit;
COSThreadTable      g_threadtable;

#if (defined(_M_IX86) && (_MSC_FULL_VER <= 13009037)) || (defined(_M_IA64) && (_MSC_FULL_VER <= 13009076))
#pragma inline_recursion(off)
#endif

ERR ErrOSTraceDeferInit();

COSThreadContext* OSThreadContext()
{
    COSThreadContext*       ptc         = NULL;

    const BOOL fTraceSavedCS = FOSSetCleanupState( fFalse );        \

    if ( ErrOSTraceDeferInit() >= JET_errSuccess )
    {
        ERR                     err         = JET_errSuccess;
        const COSThreadInfo*    pthreadinfo = NULL;

        EnterCriticalSection( &g_csThreadTable );
        pthreadinfo = g_threadtable.SeekEQ( GetCurrentThreadId() );
        ptc = pthreadinfo ? pthreadinfo->m_ptc : NULL;
        LeaveCriticalSection( &g_csThreadTable );

        if ( !pthreadinfo )
        {
            const size_t            cbAlloc     = sizeof( COSThreadContext );
            COSThreadContext* const ptcNew      = reinterpret_cast< COSThreadContext* >( LocalAlloc( 0, cbAlloc ) );
            Alloc( ptcNew );
            new( ptcNew ) COSThreadContext;

            COSThreadInfo threadinfo( GetCurrentThreadId(), ptcNew );

            EnterCriticalSection( &g_csThreadTable );
            (void)g_threadtable.ErrLoad( 1, &threadinfo );

            pthreadinfo = g_threadtable.SeekEQ( GetCurrentThreadId() );
            ptc = pthreadinfo ? pthreadinfo->m_ptc : NULL;
            LeaveCriticalSection( &g_csThreadTable );
        }
    }

HandleError:

    FOSSetCleanupState( fTraceSavedCS );

    return ptc;
}


char* OSAllocInfoString( const size_t cch )
{
    COSThreadContext* const ptc = OSThreadContext();

    return ptc ? ptc->AllocInfoString( cch ) : NULL;
}

WCHAR* OSAllocInfoStringW( const size_t cch )
{
    COSThreadContext* const ptc = OSThreadContext();

    return ptc ? ptc->AllocInfoStringW( cch ) : NULL;
}

void OSFreeInfoStrings()
{
    COSThreadContext* const ptc = OSThreadContext();

    if ( ptc )
    {
        ptc->FreeInfoStrings();
    }
}



const WCHAR         g_wszMutexTrace[]   = L"Global\\{5E5C36C0-5E7C-471f-84D7-110FDC1AFD0D}";
HANDLE              g_hMutexTrace       = NULL;
const WCHAR         g_wszFileTrace[]    = L"\\Debug\\ESE.TXT";
HANDLE              g_hFileTrace        = NULL;
LOCAL PFNTRACEEMIT  g_pfnTraceEmit      = NULL;
BOOL                g_fJetDebugTracing  = fFalse;
enum { eDateTime, eDateTimeTick, eHRT };
ULONG               g_eTraceTimeFormat  = eDateTime;

void OSTraceRegisterEmitCallback( PFNTRACEEMIT pfnTraceEmit )
{
    g_pfnTraceEmit = ( NULL != pfnTraceEmit ? pfnTraceEmit : OSTraceEmit );
}

void __stdcall OSTraceEmit( const TRACETAG tag, const char* const szPrefixNYI, const char* const szRawTrace, const ULONG_PTR ul )
{
    const size_t    cchPrefixMax                = 127;
    char            szPrefix[ cchPrefixMax + 1 ];
                    szPrefix[ cchPrefixMax ]    = 0;
    size_t          cchPrefix                   = 0;

    const size_t    cchLocalMax                 = 255;
    char            szLocal[ cchLocalMax + 1 ];
                    szLocal[ cchLocalMax ]      = 0;
    char            szEOL[]                     = "\r\n";

    size_t          cchTraceMax                 = cchLocalMax;
    char*           szTrace                     = szLocal;
    size_t          cchTrace                    = 0;

    __try
    {

        COSThreadContext* const ptc             = OSThreadContext();
        const INT               cIndentMin      = 0;
        const INT               cIndentMax      = 16;
        const INT               cchIndent       = 2;
        const INT               cIndentThread   = ptc ? ptc->Indent() : 0;


        SYSTEMTIME systemtime;
        GetLocalTime( &systemtime );

        if ( 0 == cchPrefixMax - cchPrefix )
        {
            cchPrefix = cchPrefixMax;
        }
        else
        {
            ERR errFormatPrefix = JET_errIllegalOperation;

            if ( g_eTraceTimeFormat == eDateTime )
            {
                errFormatPrefix = ErrOSStrCbFormatA(    szPrefix + cchPrefix,
                                        cchPrefixMax - cchPrefix,
                                        "[%ws %03x.%03x %04d/%02d/%02d-%02d:%02d:%02d]  ",
                                        WszUtilImageVersionName(),
                                        GetCurrentProcessId(),
                                        GetCurrentThreadId(),
                                        systemtime.wYear,
                                        systemtime.wMonth,
                                        systemtime.wDay,
                                        systemtime.wHour,
                                        systemtime.wMinute,
                                        systemtime.wSecond );
            }
            else if ( g_eTraceTimeFormat == eDateTimeTick )
            {
                errFormatPrefix = ErrOSStrCbFormatA(    szPrefix + cchPrefix,
                                        cchPrefixMax - cchPrefix,
                                        "[%ws %03x.%03x %04d/%02d/%02d-%02d:%02d:%02d.%d]  ",
                                        WszUtilImageVersionName(),
                                        GetCurrentProcessId(),
                                        GetCurrentThreadId(),
                                        systemtime.wYear,
                                        systemtime.wMonth,
                                        systemtime.wDay,
                                        systemtime.wHour,
                                        systemtime.wMinute,
                                        systemtime.wSecond,
                                        TickOSTimeCurrent() );
            }
            else if ( g_eTraceTimeFormat == eHRT )
            {
                errFormatPrefix = ErrOSStrCbFormatA(    szPrefix + cchPrefix,
                                        cchPrefixMax - cchPrefix,
                                        "[%ws %03x.%03x %I64d]  ",
                                        WszUtilImageVersionName(),
                                        GetCurrentProcessId(),
                                        GetCurrentThreadId(),
                                        HrtHRTCount() );
            }

            if ( errFormatPrefix )
            {
                cchPrefix = cchPrefixMax;
            }
            else
            {
                cchPrefix += strlen( szPrefix + cchPrefix );
            }

        }


        do  {

            cchTrace                = 0;
            szTrace[ cchTrace ]     = 0;

            const char* szLast          = NULL;
            const char* szCurr          = szRawTrace ? szRawTrace : OSTRACENULLPARAM;
            BOOL        fBOL            = TRUE;
            INT         cIndentTrace    = 0;

            while ( *szCurr )
            {
                if ( szCurr[ 0 ] == '\r' && szCurr[ 1 ] == '\r' )
                {
                    if ( szCurr[ 2 ] == '+' && szCurr[ 3 ] == '\r' )
                    {
                        cIndentTrace++;
                        szCurr += 4;
                        continue;
                    }
                    else if ( szCurr[ 2 ] == '-' && szCurr[ 3 ] == '\r' )
                    {
                        cIndentTrace--;
                        szCurr += 4;
                        continue;
                    }
                    else
                    {
                        szCurr += 2;
                        continue;
                    }
                }

                if ( fBOL )
                {
                    const INT cIndent = min( cIndentMax, max( cIndentMin, cIndentThread + cIndentTrace ) );

                    if ( (0 == cchTraceMax - cchTrace) ||
                         ErrOSStrCbFormatA( szTrace + cchTrace,
                                                cchTraceMax - cchTrace,
                                                "%-*.*s",
                                                (INT)min( cchPrefixMax, cchPrefix + cchIndent * cIndent ),
                                                (INT)cchPrefixMax,
                                                szPrefix ) )
                    {
                        cchTrace = cchTraceMax;
                    }
                    else
                    {
                        cchTrace += strlen( szTrace + cchTrace );
                    }

                    fBOL = FALSE;
                }

                szLast = szCurr;
                szCurr = szCurr + strcspn( szLast, "\r\n" );

                if ( (0 == cchTraceMax - cchTrace) ||
                     ErrOSStrCbFormatA( szTrace + cchTrace,
                                            cchTraceMax - cchTrace,
                                            "%.*s%s",
                                            (DWORD)(szCurr - szLast),
                                            szLast,
                                            szEOL ) )
                {
                    cchTrace = cchTraceMax;
                }
                else
                {
                    cchTrace += strlen( szTrace + cchTrace );
                }

                if ( szCurr[ 0 ] == '\r' && szCurr[ 1 ] != '\r' )
                {
                    szCurr++;
                }
                if ( szCurr[ 0 ] == '\n' )
                {
                    fBOL = TRUE;
                    szCurr++;
                }
            }

            if ( cchTrace == cchTraceMax )
            {
                if ( szTrace != szLocal )
                {
                    LocalFree( szTrace );
                }

                cchTraceMax = 2 * cchTraceMax;
                szTrace     = (char*)LocalAlloc( 0, cchTraceMax + 1 );
                if ( szTrace )
                {
                    szTrace[ cchTraceMax ] = 0;
                }
                cchTrace    = cchTraceMax;
            }
        }
        while ( cchTrace == cchTraceMax && szTrace );


        if ( szTrace && FOSDllUp() )
        {
            OutputDebugStringA( szTrace );


            OSEventTrace_(tag + etguidOsTraceBase,
                          1,
                          szTrace );


            if ( !g_fJetDebugTracing && g_hFileTrace )
            {
                DWORD cbT;
                WaitForSingleObjectEx( g_hMutexTrace, INFINITE, FALSE );
                const LARGE_INTEGER ibOffset = { 0, 0 };
                if ( SetFilePointerEx( g_hFileTrace, ibOffset, NULL, FILE_END ) )
                {
                    WriteFile( g_hFileTrace, szTrace, min( DWORD( -1 ), cchTrace ), &cbT, NULL );
                }
                ReleaseMutex( g_hMutexTrace );
            }

        }

        if ( szTrace != szLocal )
        {
            LocalFree( szTrace );
        }
    }
    __except( EXCEPTION_EXECUTE_HANDLER )
    {
        __try
        {
            if ( szTrace != szLocal )
            {
                LocalFree( szTrace );
            }
        }
        __except( EXCEPTION_EXECUTE_HANDLER )
        {
            AssertPREFIX( szTrace != szLocal );
        }
    }
}

void OSTrace_( const TRACETAG tag, const char* const szTrace )
{

    if ( g_fJetDebugTracing )
    {
        OSTraceEmit( tag, NULL, szTrace, g_rgtraceinfo[tag].Ul() );
    }
    (*g_pfnTraceEmit)( tag, NULL, szTrace, g_rgtraceinfo[tag].Ul() );
}

void OSTraceIndent_( const INT dLevel )
{

    COSThreadContext* const ptc = OSThreadContext();

    if ( ptc )
    {
        ptc->Indent( dLevel );
    }
}

void OSTraceSuspendGC_()
{

    COSThreadContext* const ptc = OSThreadContext();

    if ( ptc )
    {
        ptc->SuspendGC();
    }
}

void OSTraceResumeGC_()
{

    COSThreadContext* const ptc = OSThreadContext();

    if ( ptc )
    {
        ptc->ResumeGC();
    }
}



const char* OSFormat_( __format_string const char* const szFormat, __in va_list arglist )
{
    const size_t    cchLocalMax                 = 256;
    char            szLocal[ cchLocalMax ];

    size_t          cchBufferMax                = cchLocalMax;
    char*           szBuffer                    = szLocal;

    char*           szInfoString                = NULL;

    __try
    {

        size_t  cchRawMax;
        char*   szRaw;
        size_t  cchRaw;

        do  {
            cchRawMax               = cchBufferMax - 1;
            szRaw                   = szBuffer;
            szRaw[ cchRawMax ]      = 0;
            cchRaw                  = 0;

            if ( S_OK != StringCbVPrintfA( szRaw + cchRaw,
                                        ( cchRawMax - cchRaw ) * sizeof( char ),
                                        szFormat,
                                        arglist ) )
            {
                cchRaw = cchRawMax;
            }
            else
            {
                cchRaw += strlen( szRaw + cchRaw );
            }

            if ( cchRaw == cchRawMax )
            {
                if ( szBuffer != szLocal )
                {
                    LocalFree( szBuffer );
                }

                cchBufferMax    = 2 * cchBufferMax;
                szBuffer        = (char*)LocalAlloc( 0, cchBufferMax * sizeof( char ) );
            }
        }
        while ( cchRaw == cchRawMax && szBuffer );


        if ( szBuffer )
        {
            szInfoString = OSAllocInfoString( cchRaw );
            if ( szInfoString )
            {
                memcpy( szInfoString, szRaw, cchRaw * sizeof( char ) );
            }
        }

        if ( szBuffer != szLocal )
        {
            LocalFree( szBuffer );
        }
    }
    __except( EXCEPTION_EXECUTE_HANDLER )
    {
        __try
        {
            if ( szBuffer != szLocal )
            {
                LocalFree( szBuffer );
            }
        }
        __except( EXCEPTION_EXECUTE_HANDLER )
        {
            AssertPREFIX( szBuffer != szLocal );
        }
    }

    return szInfoString;
}

const WCHAR* OSFormatW_( __format_string const WCHAR* const wszFormat, __in va_list arglist )
{
    const size_t    cchLocalMax                 = 256;
    WCHAR           wszLocal[ cchLocalMax ];

    size_t          cchBufferMax                = cchLocalMax;
    WCHAR*          wszBuffer                   = wszLocal;

    WCHAR*          wszInfoString               = NULL;

    __try
    {

        size_t  cchRawMax;
        WCHAR*  wszRaw;
        size_t  cchRaw;

        do  {
            cchRawMax               = cchBufferMax - 1;
            wszRaw                  = wszBuffer;
            wszRaw[ cchRawMax ]     = 0;
            cchRaw                  = 0;

            if ( S_OK != StringCbVPrintfW( wszRaw + cchRaw,
                                        ( cchRawMax - cchRaw ) * sizeof( WCHAR ),
                                        wszFormat,
                                        arglist ) )
            {
                cchRaw = cchRawMax;
            }
            else
            {
                cchRaw += wcslen( wszRaw + cchRaw );
            }

            if ( cchRaw == cchRawMax )
            {
                if ( wszBuffer != wszLocal )
                {
                    LocalFree( wszBuffer );
                }

                cchBufferMax    = 2 * cchBufferMax;
                wszBuffer       = (WCHAR*)LocalAlloc( 0, cchBufferMax * sizeof( WCHAR ) );
            }
        }
        while ( cchRaw == cchRawMax && wszBuffer );


        if ( wszBuffer )
        {
            wszInfoString = OSAllocInfoStringW( cchRaw );
            if ( wszInfoString )
            {
                memcpy( wszInfoString, wszRaw, cchRaw * sizeof( WCHAR ) );
            }
        }

        if ( wszBuffer != wszLocal )
        {
            LocalFree( wszBuffer );
        }
    }
    __except( EXCEPTION_EXECUTE_HANDLER )
    {
        __try
        {
            if ( wszBuffer != wszLocal )
            {
                LocalFree( wszBuffer );
            }
        }
        __except( EXCEPTION_EXECUTE_HANDLER )
        {
            AssertPREFIX( wszBuffer != wszLocal );
        }
    }

    return wszInfoString;
}

#ifdef _PREFAST_
#pragma push_macro("OSFormat")
#undef OSFormat
#endif
const char* __cdecl OSFormat( __format_string const char* const szFormat, ... )
#ifdef _PREFAST_
#pragma pop_macro("OSFormat")
#endif
{
    va_list arglist;
    va_start( arglist, szFormat );

    return OSFormat_( szFormat, arglist );
}

#ifdef _PREFAST_
#pragma push_macro("OSFormatW")
#undef OSFormat
#endif
const WCHAR* __cdecl OSFormatW( __format_string const WCHAR* const wszFormat, ... )
#ifdef _PREFAST_
#pragma pop_macro("OSFormatW")
#endif
{
    va_list arglist;
    va_start( arglist, wszFormat );

    return OSFormatW_( wszFormat, arglist );
}

const char* OSFormatFileLine( const char* const szFile, const INT iLine )
{
    const char* szFilename1 = strrchr( szFile, '/' );
    const char* szFilename2 = strrchr( szFile, '\\' );
    const char* szFilename  = ( szFilename1 ?
                                    (   szFilename2 ?
                                            max( szFilename1, szFilename2 ) + 1 :
                                            szFilename1 + 1 ) :
                                    (   szFilename2 ?
                                            szFilename2 + 1 :
                                            szFile ) );

    return OSFormat( "%s(%i)", szFilename, iLine );
}

const char* OSFormatImageVersion()
{
    return OSFormat(    "%ws version %d.%02d.%04d.%04d (%ws)",
                        WszUtilImageVersionName(),
                        DwUtilImageVersionMajor(),
                        DwUtilImageVersionMinor(),
                        DwUtilImageBuildNumberMajor(),
                        DwUtilImageBuildNumberMinor(),
                        WszUtilImageBuildClass() );
}

const char* OSFormatError( const ERR err )
{
    const char*     szError         = NULL;

#ifndef MINIMAL_FUNCTIONALITY
    const char*     szErrorText     = NULL;
    JetErrorToString( err, &szError, &szErrorText );
#endif

    return szError ? szError : OSFormat( "JET API error %dL", err );
}

const char* OSFormatString( const char* sz )
{
    const char *    szFormatted;

    __try
    {
        szFormatted = ( NULL != sz ? OSFormat( "'%s'", sz ) : OSTRACENULLPARAM );
    }
    __except( EXCEPTION_EXECUTE_HANDLER )
    {
        szFormatted = NULL;
    }

    return ( NULL != szFormatted ? szFormatted : "<???>" );
}

const char* SzOSFormatStringLossyW( const WCHAR* wsz )
{
    char *  szFormatted = NULL;

    __try
    {
        
        char * szCurr;
        if ( wsz )
        {
            szFormatted = OSAllocInfoString( sizeof(char)*wcslen(wsz)+3 );
        }
        if ( szFormatted  )
        {

            szCurr = szFormatted;
            *szCurr = '\'';
            szCurr++;

            while ( *wsz )
            {
                if ( iswprint(*wsz) )
                {
                    *szCurr = (CHAR)(0x7F & *wsz);
                }
                else
                {
                    *szCurr = '?';
                }
                szCurr++;
                wsz++;
            }

            *szCurr = '\'';
            szCurr++;
            *szCurr = '\0';
            szCurr++;

        }
        else
        {
            szFormatted = const_cast<char *> (OSTRACENULLPARAM);
        }
    }
    __except( EXCEPTION_EXECUTE_HANDLER )
    {
        szFormatted = NULL;
    }

    return ( NULL != szFormatted ? szFormatted : "<???>" );
}

const char* SzOSFormatStringEscapedW( const WCHAR* wsz )
{
    const char *    szFormatted;

    AssertSz( fFalse, "NYI" );

    __try
    {
        szFormatted = ( NULL != wsz ? OSFormat( "'%ws'", wsz ) : OSTRACENULLPARAM );
    }
    __except( EXCEPTION_EXECUTE_HANDLER )
    {
        szFormatted = NULL;
    }

    return ( NULL != szFormatted ? szFormatted : "<???>" );
}

const char* SzOSFormatStringW( const WCHAR* wsz )
{
    const char *    szFormatted;

    __try
    {
        szFormatted = SzOSFormatStringLossyW(wsz);
    }
    __except( EXCEPTION_EXECUTE_HANDLER )
    {
        szFormatted = NULL;
    }

    return ( NULL != szFormatted ? szFormatted : "<???>" );
}

const char* OSFormatRawData(    const BYTE* const   rgbData,
                                const size_t        cbData,
                                const size_t        cbAddr,
                                const size_t        cbLine,
                                const size_t        cbWord,
                                const size_t        ibData )
{

    const size_t    cchAddr = 2 * cbAddr + ( cbAddr ? 1 : 0 );
    const size_t    cchHex  = 2 * cbLine + ( cbLine + cbWord - 1 ) / cbWord - 1;
    const size_t    cchChar = cbLine;
    const size_t    cchLine = cchAddr + ( cchAddr ? 2 : 0 ) + cchHex + 2 + cchChar + 1;

    char* const     szLine  = OSAllocInfoString( cchLine + 1 );
    char* const     szAddr  = szLine;
    char* const     szHex   = szLine + cchAddr + ( cchAddr ? 2 : 0 );
    char* const     szChar  = szLine + cchAddr + ( cchAddr ? 2 : 0 ) + cchHex + 2;


    static const char rgchHex[] = "0123456789abcdef";

    memset( szLine, ' ', cchLine );
    szLine[ cchLine ] = 0;

    for ( size_t ibAddr = 0; ibAddr < cbAddr; ibAddr++ )
    {
        szAddr[ 2 * ibAddr ]        = rgchHex[ ( ibData >> ( 8 * ( cbAddr - ibAddr - 1 ) + 4 ) ) & 0xf ];
        szAddr[ 2 * ibAddr + 1 ]    = rgchHex[ ( ibData >> ( 8 * ( cbAddr - ibAddr - 1 ) ) ) & 0xf ];
    }
    if ( cbAddr )
    {
        szAddr[ 2 * cbAddr ] = ':';
    }

    for ( size_t ibLine = 0; ibLine < cbLine && ibData + ibLine < cbData; ibLine += cbWord )
    {
        for ( size_t ibWord = 0; ibWord < cbWord; ibWord++ )
        {
            const size_t    ibDataRead  = ibData + ibLine + cbWord - ibWord - 1;
            BOOL            fVisible    = ibDataRead < cbData;
            char            chDataRead  = '\0';
            const size_t    ichHex      = 2 * ( ibLine + ibWord ) + ibLine / cbWord;
            const size_t    ichChar     = ibLine + cbWord - ibWord - 1;

            __try
            {
                chDataRead = rgbData[ ibDataRead ];
            }
            __except( EXCEPTION_EXECUTE_HANDLER )
            {
                fVisible = fFalse;
            }

            szHex[ ichHex ]     = !fVisible ? '?' : rgchHex[ ( chDataRead >> 4 ) & 0xf ];
            szHex[ ichHex + 1 ] = !fVisible ? '?' : rgchHex[ chDataRead & 0xf ];
            szChar[ ichChar ]   = !fVisible ? '?' : ( isprint( (BYTE)chDataRead ) && !iscntrl( (BYTE)chDataRead ) ? chDataRead : '.' );
        }
    }

    szLine[ cchLine - 1 ] = '\n';


    if ( ibData + cbLine >= cbData )
    {
        return szLine;
    }
    else
    {
        const char* szSuffix    = OSFormatRawData(  rgbData,
                                                    cbData,
                                                    cbAddr,
                                                    cbLine,
                                                    cbWord,
                                                    ibData + cbLine );
        ULONG       cbTotal     = strlen( szLine ) + strlen( szSuffix );
        char* const szTotal     = OSAllocInfoString( cbTotal );

        OSStrCbCopyA( szTotal, cbTotal, szLine );
        OSStrCbAppendA( szTotal, cbTotal, szSuffix );

        return szTotal;
    }
}

const char* OSFormatRawDataParam(
                                const BYTE* const   rgbData,
                                const size_t        cbData )
{
    if ( NULL == rgbData || 0 == cbData )
    {
        return OSTRACENULLPARAM;
    }


    static const char   rgchHex[]           = "0123456789abcdef";

    const size_t        cbDataReportable    = min( cbData, 8 );
    const size_t        cchHex              = cbDataReportable * 3;
    const size_t        cchChar             = cbDataReportable;
    const size_t        cchLine             = 1 + cchHex + 1 + cchChar + 1 + 1;

    char* const         szLine              = OSAllocInfoString( cchLine );
    char* const         szHex               = 1 + szLine;
    char* const         szChar              = szHex + cchHex + 1;

    memset( szLine, ' ', cchLine );
    szLine[0] = '<';
    szLine[ cchLine - 2 ] = '>';
    szLine[ cchLine - 1 ] = 0;

    for ( size_t ibData = 0; ibData < cbDataReportable; ibData++ )
    {
        BOOL            fVisible    = fTrue;
        char            chDataRead  = 0;
        const size_t    ichHex      = ibData * 3;
        const size_t    ichChar     = ibData;

        Assert( ichHex < cchHex );
        Assert( ichChar < cchChar );

        __try
        {
            chDataRead = rgbData[ ibData ];
        }
        __except( EXCEPTION_EXECUTE_HANDLER )
        {
            fVisible = fFalse;
        }

        if ( fVisible )
        {
            szHex[ ichHex ]     = rgchHex[ ( chDataRead >> 4 ) & 0xf ];
            szHex[ ichHex + 1 ] = rgchHex[ chDataRead & 0xf ];
            szChar[ ichChar ]   = ( isprint( (BYTE)chDataRead ) && !iscntrl( (BYTE)chDataRead ) ? chDataRead : '.' );
        }
        else
        {
            szHex[ ichHex ] = '?';
            szHex[ ichHex + 1] = '?';
            szChar[ ichChar ] = '?';
        }
    }

    return szLine;
}


void OSTracePostterm()
{

    OSTraceDestroyRefLog( ostrlSystemFixed );
}

BOOL FOSTracePreinit()
{

    OSTraceSetGlobal( fFalse );
    for ( TRACETAG itag = JET_tracetagNull + 1; itag < JET_tracetagMax; itag++ )
    {
        OSTraceSetTag( itag, fFalse );
    }
    OSTraceSetThreadidFilter( 0 );
    OSTraceRegisterEmitCallback( NULL );


    for ( TRACETAG itag = JET_tracetagNull + 1; itag < JET_tracetagMax; itag++ )
    {
        const INT       cchBuf          = 256;
        WCHAR           wszBuf[ cchBuf ];

        Assert( wcslen(g_rgwszTraceDesc[ itag ]) * sizeof( WCHAR ) < JET_tracetagDescCbMax );

        if (    FOSConfigGet( L"DEBUG/Tracing", g_rgwszTraceDesc[ itag ], wszBuf, sizeof(wszBuf) ) &&
                wszBuf[ 0 ] )
        {
            WCHAR*      wszT = NULL;
            const BOOL      fT  = !!wcstoul( wszBuf, &wszT, 0 );
            if ( !( *wszT ) )
            {
                OSTraceSetTag( itag, fT );
                if ( fT )
                {
                    OSTraceSetGlobal( fTrue );
                    OSTraceSetTag( JET_tracetagSysInitTerm, fT );
                }
            }
        }
    }

    
    
    return fTrue;
}

void OSTraceTerm()
{

    if ( g_fcsThreadTableInit )
    {
        EnterCriticalSection( &g_csThreadTable );
        g_threadtable.Clear();
        LeaveCriticalSection( &g_csThreadTable );
    }
}

ERR ErrOSTraceInit()
{

    return JET_errSuccess;
}

VOID COSLayerPreInit::DisableTracing()
{
    g_fDisableTracingForced = fTrue;
}

ERR ErrOSTraceIGetSystemWindowsDirectory( __out_ecount(uSize) LPWSTR lpBuffer, UINT uSize )
{
    ERR                             err                             = JET_errSuccess;

    if ( !GetSystemWindowsDirectoryW( lpBuffer, uSize ) )
    {
        Call( ErrERRCheck( JET_errOutOfMemory ) );
    }

HandleError:
    return err;
}

void OSTraceITerm()
{
    OSTrace( JET_tracetagSysInitTerm, OSFormat( "%s unloaded", OSFormatImageVersion() ) );

    g_threadtable.~COSThreadTable();

    if ( g_hFileTrace )
    {
        CloseHandle( g_hFileTrace );
        g_hFileTrace = NULL;
    }
    if ( g_hMutexTrace )
    {
        CloseHandle( g_hMutexTrace );
        g_hMutexTrace = NULL;
    }
    if ( g_fcsThreadTableInit )
    {
        DeleteCriticalSection( &g_csThreadTable );
        g_fcsThreadTableInit = fFalse;
    }
}

ERR ErrOSTraceIInit()
{
    ERR             err             = JET_errSuccess;
    const size_t    cchPathTrace    = MAX_PATH + 1;
    WCHAR           wszPathTrace[ cchPathTrace ];

    Assert( NULL == g_fcsThreadTableInit );
    Assert( NULL == g_hMutexTrace );
    Assert( NULL == g_hFileTrace );

    if ( !( g_fcsThreadTableInit = InitializeCriticalSectionAndSpinCount( &g_csThreadTable, 1000 ) ) )
    {
        Call( ErrOSErrFromWin32Err( GetLastError() ) );
    }

    Call( ErrOSTraceIGetSystemWindowsDirectory( wszPathTrace, cchPathTrace ) );
    OSStrCbAppendW( wszPathTrace, sizeof(wszPathTrace), g_wszFileTrace );

    if ( ( g_hFileTrace = CreateFileW(  wszPathTrace,
                                        GENERIC_WRITE,
                                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                                        NULL,
                                        OPEN_ALWAYS,
                                        FILE_ATTRIBUTE_NORMAL,
                                        NULL ) ) == INVALID_HANDLE_VALUE )
    {
        g_hFileTrace = NULL;

    }
    else
    {
        Assert( NULL != g_hFileTrace && INVALID_HANDLE_VALUE != g_hFileTrace );

        if (    !( g_hMutexTrace = CreateMutexW( NULL, FALSE, g_wszMutexTrace ) ) &&
                !( g_hMutexTrace = CreateMutexW( NULL, FALSE, wcsrchr( g_wszMutexTrace, L'\\' ) + 1 ) ) )
        {
            Call( ErrOSErrFromWin32Err( GetLastError() ) );
        }
    }

HandleError:
    AssertSz( INVALID_HANDLE_VALUE != g_hFileTrace, "g_hFileTrace should be NULL if it couldn't be opened." );
    AssertSz( ( ( NULL == g_hMutexTrace ) == ( NULL == g_hFileTrace ) )
        || err < JET_errSuccess,
        "g_hMutexTrace (%p) and g_hFileTrace (%p) must both be NULL or non-NULL. Or that there was an error.",
        g_hMutexTrace, g_hFileTrace );

    OSTrace(    JET_tracetagSysInitTerm,
                OSFormat(   "%s loaded.\r\n"
                            OSTRACEINDENTSZ
                            "Image:              \"%ws\"\r\n"
                            "Process:            \"%ws\"\r\n"
                            "Trace File:         \"%ws\"%s\r\n"
                            OSTRACEUNINDENTSZ,
                            OSFormatImageVersion(),
                            WszUtilImagePath(),
                            WszUtilProcessPath(),
                            ( g_pfnTraceEmit == OSTraceEmit ? wszPathTrace : L"<not used>" ),
                            ( g_pfnTraceEmit == OSTraceEmit && NULL == g_hFileTrace ? " (open failed)" : "" ) ) );

    if ( err < JET_errSuccess )
    {
        if ( g_hFileTrace )
        {
            CloseHandle( g_hFileTrace );
            g_hFileTrace = NULL;
        }
        if ( g_hMutexTrace )
        {
            CloseHandle( g_hMutexTrace );
            g_hMutexTrace = NULL;
        }
        if ( g_fcsThreadTableInit )
        {
            DeleteCriticalSection( &g_csThreadTable );
            g_fcsThreadTableInit = fFalse;
        }
    }
    return err;
}


class COSTraceDeferInit
{
    public:

        COSTraceDeferInit();
        ~COSTraceDeferInit();

        ERR ErrInit();

    private:

        HANDLE              m_hEventInit;
        DWORD               m_tidInit;
        ERR                 m_errInit;
        BOOL                m_fInit;
};

COSTraceDeferInit::COSTraceDeferInit()
    :   m_hEventInit( CreateEventW( NULL, TRUE, FALSE, NULL ) ),
        m_tidInit( 0 ),
        m_errInit( m_hEventInit ? JET_errSuccess : JET_errOutOfMemory ),
        m_fInit( m_hEventInit ? fFalse : fTrue )
{
}

COSTraceDeferInit::~COSTraceDeferInit()
{
    OSTraceITerm();

    if ( m_hEventInit )
    {
        CloseHandle( m_hEventInit );
    }
}

ERR COSTraceDeferInit::ErrInit()
{
    if ( !m_fInit )
    {
        if ( InterlockedCompareExchange( (volatile LONG*)&m_tidInit, GetCurrentThreadId(), 0 ) == 0 )
        {
            m_errInit   = ErrOSTraceIInit();
            m_fInit     = fTrue;
            SetEvent( m_hEventInit );
        }
        if ( m_tidInit != GetCurrentThreadId() )
        {
            WaitForSingleObjectEx( m_hEventInit, INFINITE, FALSE );
        }
    }

    return m_errInit;
}

COSTraceDeferInit g_ostracedeferinit;

ERR ErrOSTraceDeferInit()
{
    return g_ostracedeferinit.ErrInit();
}

#ifndef MINIMAL_FUNCTIONALITY





CFastTraceLogBuffer::CFastTraceLogBuffer() :
    m_ibOffset( 0 ),
    m_dwTIDLast( 0 ),
    m_cbHeader( sizeof(TICK) ),
    m_fTracingDisabled( fFalse ),
    m_pfnErrFlushBuffer( NULL ),
    m_pvFlushBufferContext( NULL ),
    m_crit( CLockBasicInfo( CSyncBasicInfo( "FTLBuffer" ), rankFTLBuffer, 0 ) )
{
    Assert( !FFTLBInitialized() );
}

ERR CFastTraceLogBuffer::ErrFTLBInit( PfnErrFTLBFlushBuffer pfnFlushBuffer, void * pvFlushBufferContext, const TICK tickInitialBase )
{
    C_ASSERT( ftltidSmallMax == ftltidShortMac + 1 );


    Assert( !FFTLBInitialized() );

    memset( m_rgbBuffer, 0, sizeof(m_rgbBuffer) );


    FTLBISetTickBase( tickInitialBase );
    Assert( tickInitialBase == TickFTLBITickBase() );

    m_ibOffset = m_cbHeader;

    m_pvFlushBufferContext = pvFlushBufferContext;


    m_pfnErrFlushBuffer = pfnFlushBuffer;

    Assert( FFTLBInitialized() );

    return JET_errSuccess;
}


void CFastTraceLogBuffer::FTLBTerm( const BOOL fAbrubt )
{
    Assert( FFTLBInitialized() );


    m_crit.Enter();

    
    memset( m_rgbBuffer + m_ibOffset, 0, _countof( m_rgbBuffer ) - m_ibOffset );

    if( !fAbrubt )
    {

        (void)m_pfnErrFlushBuffer( m_pvFlushBufferContext, m_rgbBuffer, _countof( m_rgbBuffer ) );
    }
    
    m_ibOffset = 0;


    m_pvFlushBufferContext = NULL;
    m_pfnErrFlushBuffer = NULL;

    Assert( !FFTLBInitialized() );

    m_crit.Leave();
}

CFastTraceLogBuffer::~CFastTraceLogBuffer()
{
    Assert( !FFTLBInitialized() || FUtilProcessAbort() );
}


#ifdef DEBUG
const BYTE * CFastTraceLogBuffer::s_pbPrevPrevPrev;
const BYTE * CFastTraceLogBuffer::s_pbPrevPrev;
const BYTE * CFastTraceLogBuffer::s_pbPrev;
#endif


const BYTE * CFastTraceLogBuffer::PbFTLBParseTraceTick( __in const BYTE fTickInfo, const BYTE * pbTrace, __in const TICK tickBase, __out TICK * const ptick )
{
    TICK tickTrace = 0;

    
    switch ( fTickInfo )
    {
        case bHeaderTickHeaderMatch:
            tickTrace = tickBase;
            pbTrace += 0;
            break;
        case bHeaderTickDelta:
            tickTrace = tickBase + pbTrace[0];
            pbTrace += 1;
            break;
        case bHeaderTickRaw:
            tickTrace = *( reinterpret_cast<UnalignedLittleEndian<const TICK>*>( (BYTE*)pbTrace ) );
            pbTrace += 4;
            break;

        default:
        case bHeaderTickReserved:
            AssertSz( fFalse, "Is this trace file corrupt?" );
            pbTrace = NULL;
            break;
    }

    if ( ptick )
    {
        *ptick = tickTrace;
    }

    return pbTrace;
}


ERR CFastTraceLogBuffer::ErrFTLBParseTraceHeader( const BYTE * pbTrace, __out FTLTID * const pftltid, __in const TICK tickBase, __out TICK * const ptick )
{
#ifdef DEBUG
    s_pbPrevPrevPrev = s_pbPrevPrev;
    s_pbPrevPrev = s_pbPrev;
    s_pbPrev = pbTrace;
#endif


    Assert( 0 == ( ~( mskHeaderShortTraceID | mskHeaderTick ) & pbTrace[0] ) );
    
    BYTE bTraceIDShort  = pbTrace[0] & mskHeaderShortTraceID;
    BYTE fTickInfo      = pbTrace[0] & mskHeaderTick;

    if ( bTraceIDShort == 0 )
    {
        return ErrERRCheck( errNotFound );
    }

    pbTrace += cbTraceHeader;


    if ( bTraceIDShort != bHeaderLongTraceID )
    {
        *pftltid = (USHORT)bTraceIDShort;
        Assert( *pftltid <= ftltidShortMac );
    }
    else
    {
        C_ASSERT( bHeaderLongTraceID == mskHeaderShortTraceID );
        Assert( bTraceIDShort == bHeaderLongTraceID );
        CompEndianLowSpLos16b   ce_ftltid( pbTrace, 2 );
        *pftltid = ce_ftltid.Us();
        pbTrace += ce_ftltid.Cb();
    }


    pbTrace = PbFTLBParseTraceTick( fTickInfo, pbTrace, tickBase, ptick );
    if ( NULL == pbTrace )
    {
        return ErrERRCheck( JET_errLogCorrupted );
    }
    
    return JET_errSuccess;
}


ERR CFastTraceLogBuffer::ErrFTLBParseTraceData( const BYTE * pbTrace, __in const FTLTID ftltid, __in const FTLTDESC ftltdesc, __out ULONG * pcbTraceData, __out const BYTE ** ppbTraceData )
{
    Assert( 0 == ( ~( mskHeaderShortTraceID | mskHeaderTick ) & pbTrace[0] ) );

    const BYTE bTraceHeader     = pbTrace[0];


    pbTrace += cbTraceHeader;

    if ( bHeaderLongTraceID == ( bTraceHeader & mskHeaderShortTraceID ) )
    {
        CompEndianLowSpLos16b   ce_ftltid( pbTrace, 2 );
        Expected( ce_ftltid.Cb() == 1 || ce_ftltid.Cb() == 2 );
        pbTrace += ce_ftltid.Cb();
    }

    
    pbTrace = PbFTLBParseTraceTick( bTraceHeader & mskHeaderTick, pbTrace, 0x0, NULL );
    if ( NULL == pbTrace )
    {
        return ErrERRCheck( JET_errLogCorrupted );
    }



    if ( CbFTLBIFixedSize( ftltdesc ) )
    {
        *pcbTraceData = CbFTLBIFixedSize( ftltdesc );
        pbTrace += 0;
    }
    else
    {
        CompEndianLowSpLos16b   ce_cbExtra( pbTrace, 2 );
        *pcbTraceData = ce_cbExtra.Us();
        pbTrace += ce_cbExtra.Cb();
    }

    *ppbTraceData = pbTrace;

    return JET_errSuccess;
}





void CFastTraceLog::FTLIResetWriteBuffering( void )
{
    Assert( m_pfapiTraceLog == NULL );
    Assert( m_pbTraceLogHeader == NULL );
    Assert( m_semBuffersInUse.CAvail() == 0 );

    C_ASSERT( _countof(m_rgfBufferState) == _countof(m_rgpbWriteBuffers) );

    m_ibWriteBufferCurrent = NULL;
    m_ipbWriteBufferCurrent = ibufUninitialized;
    m_cbWriteBufferFull = 0;
}

class CFTLFileSystemConfiguration : public CDefaultFileSystemConfiguration
{
    public:
        CFTLFileSystemConfiguration()
        {
            m_dtickAccessDeniedRetryPeriod = 50;
        }
} g_fsconfigFTL;

CFastTraceLog::CFastTraceLog(   const FTLDescriptor * const         pftldesc,
                                IFileSystemConfiguration * const    pfsconfig ) :
    m_ftlb(),
    m_fTerminating( fFalse ),

    m_cbWriteBufferMax( 0 ),
    m_piorTraceLog( NULL ),

    m_pfsconfig( pfsconfig ? pfsconfig : &g_fsconfigFTL ),
    m_pfsapi( NULL ),
    m_pfapiTraceLog( NULL ),
    m_pbTraceLogHeader( NULL ),


    m_ibWriteBufferCurrent( 0 ),
    m_ipbWriteBufferCurrent( 0 ),
    m_cbWriteBufferFull( 0 ),
    m_sxwlFlushing( CLockBasicInfo( CSyncBasicInfo( "FTLFlush" ), rankFTLFlush, CLockDeadlockDetectionInfo::subrankNoDeadlock ) ),

    m_semBuffersInUse( CLockBasicInfo( CSyncBasicInfo( "FTLFlushBuffs" ), rankFTLFlushBuffs, CLockDeadlockDetectionInfo::subrankNoDeadlock ) ),

    m_cOutstandingIO( 0 ),
    m_cOutstandingIOHighWater( 0 ),

    m_pftlr( NULL )
{


    memset( &m_ftldesc, 0, sizeof ( m_ftldesc ) );
    memset( &m_rgfBufferState, 0, sizeof ( m_rgfBufferState ) );
    memset( &m_rgpbWriteBuffers, 0, sizeof ( m_rgpbWriteBuffers ) );
    C_ASSERT( ibPrivateHeaderOffset == 1024 );


    Assert( m_semBuffersInUse.CAvail() == 0 );

    FTLIResetWriteBuffering();


    const ULONG ulSchemaIDUnspec = 0x80000000;

    if ( pftldesc )
    {
        Assert( pftldesc->m_ulSchemaID != ulSchemaIDUnspec );

        m_ftldesc = *pftldesc;
    }
    else
    {

        m_ftldesc.m_ulSchemaID = ulSchemaIDUnspec;
        m_ftldesc.m_ulSchemaVersionMajor = ulFTLVersionMajor;
        m_ftldesc.m_ulSchemaVersionMinor = ulFTLVersionMinor;
        m_ftldesc.m_ulSchemaVersionUpdate = ulFTLVersionUpdate;

        m_ftldesc.m_cShortTraceDescriptors = 0;
        m_ftldesc.m_cLongTraceDescriptors = 0;
        memset( m_ftldesc.m_rgftltdescShortTraceDescriptors, 0, sizeof(m_ftldesc.m_rgftltdescShortTraceDescriptors) );
        m_ftldesc.m_rgftltdescLongTraceDescriptors = NULL;
    }
}


ERR CFastTraceLog::ErrFTLIFlushHeader()
{
    ERR err = JET_errSuccess;

    Assert( m_pfapiTraceLog );

    TraceContextScope tcFtlFlush;
    tcFtlFlush->iorReason = *m_piorTraceLog;
    Call( m_pfapiTraceLog->ErrIOWrite( *tcFtlFlush, 0, CbFullHeader( ), m_pbTraceLogHeader, qosIONormal ) );

HandleError:

    if ( err < JET_errSuccess )
    {
        PftlhdrFTLTraceLogHeader()->le_cWriteFailures = PftlhdrFTLTraceLogHeader()->le_cWriteFailures + 1;
    }

    return err;
}

void CFastTraceLog::SetFTLDisabled()
{
    m_ftlb.SetFTLBTracingDisabled();
}

ERR CFastTraceLog::ErrFTLICheckVersions()
{

    if ( PftlhdrFTLTraceLogHeader()->le_filetype != JET_filetypeFTL )
    {
        return ErrERRCheck( JET_errLogCorrupted );
    }

    if ( PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMajor != ulFTLVersionMajor )
    {
        return ErrERRCheck( JET_errBadLogVersion );
    }
    if ( PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMinor > ulFTLVersionMinor )
    {
        return ErrERRCheck( JET_errBadLogVersion );
    }

    if ( PftlhdrFTLTraceLogHeader()->le_ulSchemaID != m_ftldesc.m_ulSchemaID )
    {
        return ErrERRCheck( JET_errBadLogVersion );
    }

    if ( PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMajor != m_ftldesc.m_ulSchemaVersionMajor )
    {
        return ErrERRCheck( JET_errBadLogVersion );
    }
    if ( PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMinor > m_ftldesc.m_ulSchemaVersionMinor )
    {
        return ErrERRCheck( JET_errBadLogVersion );
    }

    return JET_errSuccess;
}


ERR CFastTraceLog::ErrFTLInitWriter( __in_z const WCHAR * wszTraceLogFile, IOREASON * pior, __in const FTLInitFlags ftlif )
{
    ERR err = JET_errSuccess;
    TraceContextScope tcFtl;
    tcFtl->iorReason = *pior;

    Assert( FOSLayerUp() );

    m_fTerminating = fFalse;

    m_cbWriteBufferMax = m_pfsconfig->CbMaxWriteSize();

    FTLInitFlags ftlifOpenFileDisp = FTLInitFlags( ftlifmskFileDisposition & ftlif );

    m_piorTraceLog = pior;

    if ( m_pfsapi == NULL )
    {
        Call( ErrOSFSCreate( m_pfsconfig, &m_pfsapi ) );
    }


    err = m_pfsapi->ErrFileCreate(  wszTraceLogFile,
                                    (   ftlifReCreateOverwrite == ftlifOpenFileDisp ?
                                            IFileAPI::fmfOverwriteExisting :
                                            IFileAPI::fmfNone ) |
                                        ( IFileAPI::fmfLossyWriteBack ),
                                    &m_pfapiTraceLog );

    ftlifOpenFileDisp = ( err >= JET_errSuccess ) ? ftlifNewlyCreated : ftlifNone;

    if ( err == JET_errFileAlreadyExists )
    {
        Assert( ftlifReCreateOverwrite != ftlifOpenFileDisp );

        err = m_pfsapi->ErrFileOpen( wszTraceLogFile, IFileAPI::fmfLossyWriteBack, &m_pfapiTraceLog );
        ftlifOpenFileDisp = ( err >= JET_errSuccess ) ? ftlifReOpenExisting : ftlifOpenFileDisp;
    }
    Call( err );

    Assert( ftlifOpenFileDisp == ftlifNewlyCreated || ftlifOpenFileDisp == ftlifReOpenExisting );
    

    CallS( m_ftlb.ErrFTLBInit( CFastTraceLog::ErrFTLFlushBuffer, this ) );


    C_ASSERT( sizeof(FTLFILEHDR) < ibPrivateHeaderOffset );
    Alloc( m_pbTraceLogHeader = (BYTE*)PvOSMemoryPageAlloc( CbFullHeader(), NULL ) );
    Assert( PftlhdrFTLTraceLogHeader() );


    switch( ftlifOpenFileDisp )
    {
        case ftlifReCreateOverwrite:
            AssertSz( fFalse, "Should've been translated above" );
        case ftlifNewlyCreated:
            memset( m_pbTraceLogHeader, 0, CbFullHeader() );
            PftlhdrFTLTraceLogHeader()->le_filetype = JET_filetypeFTL;
            PftlhdrFTLTraceLogHeader()->le_tracelogstate = eFTLCreated;
            PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMajor = ulFTLVersionMajor;
            PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMinor = ulFTLVersionMinor;
            PftlhdrFTLTraceLogHeader()->le_ulFTLVersionUpdate = ulFTLVersionUpdate;

            PftlhdrFTLTraceLogHeader()->le_ftFirstOpen = UtilGetCurrentFileTime();

            PftlhdrFTLTraceLogHeader()->le_ulSchemaID = m_ftldesc.m_ulSchemaID;
            PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMajor = m_ftldesc.m_ulSchemaVersionMajor;
            PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMinor = m_ftldesc.m_ulSchemaVersionMinor;
            PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionUpdate = m_ftldesc.m_ulSchemaVersionUpdate;

            Call( m_pfapiTraceLog->ErrSetSize( *tcFtl, CbFullHeader(), fTrue, qosIONormal ) );

            PftlhdrFTLTraceLogHeader()->le_cbLastKnownBuffer = 2 * CbFullHeader();

            break;

        case ftlifReOpenExisting:
            Call( m_pfapiTraceLog->ErrIORead( *tcFtl, 0, CbFullHeader(), m_pbTraceLogHeader, qosIONormal ) );

            Assert( PftlhdrFTLTraceLogHeader()->le_filetype == JET_filetypeFTL );

            Call( ErrFTLICheckVersions() );

            switch( PftlhdrFTLTraceLogHeader()->le_tracelogstate )
            {
                case eFTLClean:
                    PftlhdrFTLTraceLogHeader()->le_cReOpens = PftlhdrFTLTraceLogHeader()->le_cReOpens + 1;
                    break;
                case eFTLDirty:
                    PftlhdrFTLTraceLogHeader()->le_cRecoveries = PftlhdrFTLTraceLogHeader()->le_cRecoveries + 1;
                    break;
                case eFTLCreated:
                default:
                    AssertSz( fFalse, "Log file in a bad state (%d)\n", (FASTTRACELOGSTATE)PftlhdrFTLTraceLogHeader()->le_tracelogstate );
                    Error( ErrERRCheck( JET_errLogCorrupted ) );
            }
            PftlhdrFTLTraceLogHeader()->le_ftLastOpen = UtilGetCurrentFileTime();
            PftlhdrFTLTraceLogHeader()->le_ftLastClose = UtilGetCurrentFileTime();

            
            break;

        default:
            AssertSz( fFalse, "Unknown file disposition = %d\n", ftlifOpenFileDisp );
            Error( ErrERRCheck( errCodeInconsistency ) );
    }


    Assert( PftlhdrFTLTraceLogHeader()->le_cbLastKnownBuffer );
    if ( PftlhdrFTLTraceLogHeader()->le_cbLastKnownBuffer == 0 )
    {
        Error( ErrERRCheck( JET_errLogCorrupted ) );
    }



    if ( ( m_ftlb.CbFTLBBuffer() > (ULONG)m_cbWriteBufferMax ) ||
            ( ( m_cbWriteBufferMax % m_ftlb.CbFTLBBuffer() ) != 0 ) )
    {
        Call( ErrERRCheck( JET_errInvalidBufferSize ) );
    }

    PftlhdrFTLTraceLogHeader()->le_tracelogstate = eFTLDirty;

    CallS( ErrFTLICheckVersions() );

    Call( ErrFTLIFlushHeader() );


    m_ibWriteBufferCurrent = PftlhdrFTLTraceLogHeader()->le_cbLastKnownBuffer;
    Assert( m_ibWriteBufferCurrent != 0 );


    Assert( m_semBuffersInUse.CAvail() == 0 );
    m_semBuffersInUse.Release( _countof(m_rgpbWriteBuffers) );

HandleError:

    if ( err < JET_errSuccess )
    {
        Assert( m_semBuffersInUse.CAvail() == 0 );

        if ( m_pbTraceLogHeader )
        {
            OSMemoryPageFree( m_pbTraceLogHeader );
            m_pbTraceLogHeader = NULL;
        }
        if ( m_pfapiTraceLog )
        {
            m_pfapiTraceLog->SetNoFlushNeeded();
            delete m_pfapiTraceLog;
            m_pfapiTraceLog = NULL;
        }

        if ( m_ftlb.FFTLBInitialized() )
        {
            m_ftlb.FTLBTerm( fTrue  );
        }
        
        SetFTLDisabled();
    }

    Assert( NULL == m_pftlr );

    return err;
}


ERR CFastTraceLog::ErrFTLInitReader( __in_z const WCHAR * wszTraceLogFile, IOREASON * pior, __in const FTLInitFlags ftlif, __out CFTLReader ** ppftlr )
{
    ERR err = JET_errSuccess;
    TraceContextScope tcFtl;
    tcFtl->iorReason = *pior;

    m_piorTraceLog = pior;

    if ( m_pfsapi == NULL )
    {
        Call( ErrOSFSCreate( m_pfsconfig, &m_pfsapi ) );
    }


    Call( m_pfsapi->ErrFileOpen( wszTraceLogFile, IFileAPI::fmfLossyWriteBack, &m_pfapiTraceLog ) );

    Alloc( m_pbTraceLogHeader = (BYTE*)PvOSMemoryPageAlloc( CbFullHeader(), NULL ) );
    Assert( PftlhdrFTLTraceLogHeader() );

    Call( m_pfapiTraceLog->ErrIORead( *tcFtl, 0, CbFullHeader( ), m_pbTraceLogHeader, qosIONormal ) );


    if ( ppftlr )
    {
        Alloc( m_pftlr = new CFastTraceLog::CFTLReader( this, ftlif & CFastTraceLog::ftlifKeepStats ) );

        *ppftlr = m_pftlr;
    }

HandleError:

    if ( err < JET_errSuccess )
    {
        if ( m_pftlr )
        {
            delete m_pftlr;
        }
        
        if ( m_pfapiTraceLog )
        {
            delete m_pfapiTraceLog;
        }
    }

    Assert( 0 == m_ibWriteBufferCurrent );

    return err;
}

ERR CFastTraceLog::ErrFTLSetPostProcessHeader( void * pv1kHeader )
{
    Assert( ( ibPrivatePostProcessedHeaderOffset + CbPrivateHeader() ) <= CbFullHeader() );
    memcpy( m_pbTraceLogHeader + ibPrivatePostProcessedHeaderOffset, pv1kHeader, CbPrivateHeader() );

    PftlhdrFTLTraceLogHeader()->le_tracelogstate = eFTLPostProcessed;
    
    return ErrFTLIFlushHeader();
}

void CFastTraceLog::FTLTerm()
{

    m_fTerminating = fTrue;

    if ( m_ftlb.FFTLBInitialized() )
    {
        m_ftlb.FTLBTerm();

        Assert( m_ibWriteBufferCurrent );
        PftlhdrFTLTraceLogHeader()->le_cbLastKnownBuffer = m_ibWriteBufferCurrent;
    }

    if ( m_pfapiTraceLog )
    {

        if ( m_ibWriteBufferCurrent )
        {


            for( ULONG iBuffer = 0; iBuffer < _countof(m_rgpbWriteBuffers); iBuffer++ )
            {
                m_semBuffersInUse.Acquire();
            }
            Assert( m_semBuffersInUse.CAvail() == 0 );


            ULONG cBuffersUsed = 0;
            C_ASSERT( _countof(m_rgpbWriteBuffers) == _countof(m_rgfBufferState) );
            for( ULONG iBuffer = 0; iBuffer < _countof(m_rgpbWriteBuffers); iBuffer++ )
            {
                Assert( m_rgfBufferState[iBuffer] == fBufferAvailable || m_rgfBufferState[iBuffer] == fBufferDone );
                if ( m_rgfBufferState[iBuffer] == fBufferDone )
                {
                    cBuffersUsed++;
                }
                if ( m_rgpbWriteBuffers[iBuffer] )
                {
                    OSMemoryPageFree( m_rgpbWriteBuffers[iBuffer] );
                    m_rgpbWriteBuffers[iBuffer] = NULL;
                }
                m_rgfBufferState[iBuffer] = fBufferAvailable;
            }


            if ( PftlhdrFTLTraceLogHeader()->le_cMaxWriteIOs < (ULONG)m_cOutstandingIOHighWater )
            {
                PftlhdrFTLTraceLogHeader()->le_cMaxWriteIOs = m_cOutstandingIOHighWater;
            }
            if ( PftlhdrFTLTraceLogHeader()->le_cMaxWriteBuffers < cBuffersUsed )
            {
                PftlhdrFTLTraceLogHeader()->le_cMaxWriteBuffers = cBuffersUsed;
            }

        }


        if ( PftlhdrFTLTraceLogHeader() )
        {
            PftlhdrFTLTraceLogHeader()->le_tracelogstate = eFTLClean;
            
            CallS( ErrFTLICheckVersions() );

            (void)ErrFTLIFlushHeader();
        }


        m_pfapiTraceLog->SetNoFlushNeeded();
        delete m_pfapiTraceLog;
        m_pfapiTraceLog = NULL;
    }

    delete m_pfsapi;
    m_pfsapi = NULL;


    if ( m_pbTraceLogHeader )
    {
        OSMemoryPageFree( m_pbTraceLogHeader );
        m_pbTraceLogHeader = NULL;
    }
    Assert( NULL == PftlhdrFTLTraceLogHeader() );


    if ( m_pftlr )
    {
        delete m_pftlr;
        m_pftlr = NULL;
    }

    FTLIResetWriteBuffering();
}

TICK CFastTraceLog::TickFTLTickBaseCurrent() const
{
    return m_ftlb.TickFTLBITickBase();
}

INT CFastTraceLog::IFTLIGetFlushBuffer()
{
    Assert( m_semBuffersInUse.CAvail() < _countof(m_rgpbWriteBuffers) );

    const INT iEndBuffer = m_ipbWriteBufferCurrent;


    INT iFastTry = ( iEndBuffer - 5 < 0 ) ? 0 : iEndBuffer - 5;
    INT fCurrBuff = m_rgfBufferState[iFastTry];
    if ( fCurrBuff == fBufferAvailable || fCurrBuff == fBufferDone )
    {
        const INT fPre = AtomicCompareExchange( (LONG*)&(m_rgfBufferState[iFastTry]), fCurrBuff, fBufferInUse );
        if ( fPre == fCurrBuff )
        {
            if ( m_rgpbWriteBuffers[iFastTry] == NULL )
            {
                Assert( m_cbWriteBufferMax );
                const BOOL fCleanUpStateSaved = FOSSetCleanupState( fFalse );
                m_rgpbWriteBuffers[iFastTry] = (BYTE*)PvOSMemoryPageAlloc( m_cbWriteBufferMax, NULL );
                (void)FOSSetCleanupState( fCleanUpStateSaved );
                if ( m_rgpbWriteBuffers[iFastTry] == NULL )
                {
                    return ibufUninitialized;
                }
            }

            memset( m_rgpbWriteBuffers[iFastTry], 0, m_cbWriteBufferMax );

            return iFastTry;
        }
    }

    for( INT iBuffer = IrrNext( iEndBuffer, _countof(m_rgpbWriteBuffers) );
            iBuffer != iEndBuffer;
            iBuffer = IrrNext( iBuffer, _countof(m_rgpbWriteBuffers) ) )
    {
        fCurrBuff = m_rgfBufferState[iBuffer];
        if ( fCurrBuff == fBufferAvailable || fCurrBuff == fBufferDone )
        {
            const INT fPre = AtomicCompareExchange( (LONG*)&(m_rgfBufferState[iBuffer]), fCurrBuff, fBufferInUse );
            if ( fPre == fCurrBuff )
            {
                if ( m_rgpbWriteBuffers[iBuffer] == NULL )
                {
                    const BOOL fCleanUpStateSaved = FOSSetCleanupState( fFalse );
                    m_rgpbWriteBuffers[iBuffer] = (BYTE*)PvOSMemoryPageAlloc( m_cbWriteBufferMax, NULL );
                    (void)FOSSetCleanupState( fCleanUpStateSaved );
                    if ( m_rgpbWriteBuffers[iBuffer] == NULL )
                    {
                        return ibufUninitialized;
                    }
                }

                memset( m_rgpbWriteBuffers[iBuffer], 0, m_cbWriteBufferMax );

                return iBuffer;
            }
        }
    }


    AssertSz( fFalse, "We shouldn't be here, otherwise the m_semBuffersInUse isn't protecting us properly or we're not releasing at the right time or something." );

    return ibufUninitialized;
}

void CFastTraceLog::FTLFlushBufferComplete( const ERR           err,
                                IFileAPI* const     pfapi,
                                const FullTraceContext& ptc,
                                const OSFILEQOS     grbitQOS,
                                const QWORD         ibOffset,
                                const DWORD         cbData,
                                const BYTE* const       pbData,
                                const DWORD_PTR     keyIOComplete )
{
    CFastTraceLog * pftl = (CFastTraceLog *)keyIOComplete;

    ULONG iBuffer;
    for( iBuffer = 0; iBuffer < _countof(pftl->m_rgpbWriteBuffers); iBuffer++ )
    {
        if ( pftl->m_rgpbWriteBuffers[iBuffer] == pbData )
        {
            break;
        }
    }
    Assert( iBuffer < _countof(pftl->m_rgpbWriteBuffers) );

    if ( err < JET_errSuccess )
    {
        pftl->PftlhdrFTLTraceLogHeader()->le_cWriteFailures = pftl->PftlhdrFTLTraceLogHeader()->le_cWriteFailures + 1;
    }


    const INT fPre = AtomicCompareExchange( (LONG*)&(pftl->m_rgfBufferState[iBuffer]), fBufferFlushed, fBufferDone );
    Assert( fPre == fBufferFlushed );

    AtomicDecrement( (LONG*)&(pftl->m_cOutstandingIO) );


    pftl->m_semBuffersInUse.Release();
}

ERR CFastTraceLog::ErrFTLIFlushBuffer( __in_bcount(cbBuffer) const BYTE * rgbBuffer, __in const INT cbBuffer, const BOOL fTermForceFlush )
{

Retry:

    m_sxwlFlushing.AcquireSharedLatch();


    INT cbPre = AtomicExchangeAdd( (LONG*)(&m_cbWriteBufferFull), 0 );


    INT ipbCurrent = AtomicExchangeAdd( (LONG*)(&m_ipbWriteBufferCurrent), 0 );

    if ( ibufUninitialized == ipbCurrent )
    {

        m_sxwlFlushing.ReleaseSharedLatch();
        m_sxwlFlushing.AcquireWriteLatch();

        ipbCurrent = m_ipbWriteBufferCurrent;
        if ( ibufUninitialized != ipbCurrent )
        {
            goto Retry;
        }


        m_semBuffersInUse.Acquire();

        Assert( ibufUninitialized == ipbCurrent );

        ipbCurrent = IFTLIGetFlushBuffer();
        if ( ipbCurrent == ibufUninitialized )
        {
            m_semBuffersInUse.Release();
            m_sxwlFlushing.ReleaseWriteLatch();
            PftlhdrFTLTraceLogHeader()->le_cWriteFailures = PftlhdrFTLTraceLogHeader()->le_cWriteFailures + 1;
            return ErrERRCheck( JET_errOutOfMemory );
        }
        Assert( m_rgfBufferState[ipbCurrent] == fBufferInUse );

#ifdef USE_ATOMIC_THAT_SEEMS_LIKE_IT_TRIGGERS_COMPILER_BUG
        const INT ipbPre = AtomicCompareExchange( (LONG*)&m_ipbWriteBufferCurrent, ipbCurrent, ibufUninitialized );
        Assert( ipbPre == ibufUninitialized );
#else
        Expected( m_ipbWriteBufferCurrent == ibufUninitialized );
        m_ipbWriteBufferCurrent = ipbCurrent;
#endif
        Assert( m_cbWriteBufferFull == 0 );

        m_sxwlFlushing.DowngradeWriteLatchToSharedLatch();
    }

    Assert( m_rgfBufferState[ipbCurrent] == fBufferInUse );


    if ( cbPre + cbBuffer > m_cbWriteBufferMax )
    {
        m_sxwlFlushing.ReleaseSharedLatch();
        UtilSleep( 1 );
        goto Retry;
    }
    
    const INT ibOffset = AtomicCompareExchange( (LONG*)&m_cbWriteBufferFull, cbPre, cbPre + cbBuffer );
    if ( ibOffset != cbPre )
    {
        m_sxwlFlushing.ReleaseSharedLatch();
        UtilSleep( 1 );
        goto Retry;
    }
    Assert( ibOffset == cbPre );
    Assert( ibOffset >= 0 );
    Assert( ibOffset + cbBuffer <= m_cbWriteBufferMax );

    Assert( m_rgfBufferState[ipbCurrent] == fBufferInUse );


    const BOOL fAcquiredBufferFill = ( ibOffset + cbBuffer == m_cbWriteBufferMax );

    Assert( ( ibOffset + cbBuffer ) <= m_cbWriteBufferMax );
    Assert( ( ibOffset + cbBuffer ) < m_cbWriteBufferMax || fAcquiredBufferFill );
    Assert( ( ibOffset + cbBuffer ) == m_cbWriteBufferMax || !fAcquiredBufferFill );

    Assert( m_rgfBufferState[ipbCurrent] == fBufferInUse );


    if ( fTermForceFlush || fAcquiredBufferFill )
    {
        CSXWLatch::ERR errUpgrade = m_sxwlFlushing.ErrUpgradeSharedLatchToExclusiveLatch();
        Assert( errUpgrade != CSXWLatch::ERR::errLatchConflict );
        if ( errUpgrade == CSXWLatch::ERR::errWaitForExclusiveLatch )
        {
            AssertSz( fFalse, "I don't even think we could get into a situation where we wait here.  But I could be wrong, so wait anyway." );
            m_sxwlFlushing.WaitForExclusiveLatch();
            errUpgrade = CSXWLatch::ERR::errSuccess;
        }
        Assert( errUpgrade == CSXWLatch::ERR::errSuccess );

        if ( !fTermForceFlush )
        {
            Assert( fAcquiredBufferFill );


            m_semBuffersInUse.Acquire();

            const INT ipbNext = IFTLIGetFlushBuffer();
            if ( ipbNext == ibufUninitialized )
            {
                m_semBuffersInUse.Release();
                return ErrERRCheck( JET_errOutOfMemory );
            }

            const INT ipbPre = AtomicCompareExchange( (LONG*)&m_ipbWriteBufferCurrent, ipbCurrent, ipbNext );
            Assert( ipbPre == ipbCurrent );

            cbPre = AtomicCompareExchange( (LONG*)&m_cbWriteBufferFull, m_cbWriteBufferMax, 0 );
            Assert( cbPre == m_cbWriteBufferMax );
        }
    }


    memcpy( m_rgpbWriteBuffers[ipbCurrent] + ibOffset, rgbBuffer, cbBuffer );


    Assert( m_rgfBufferState[ipbCurrent] == fBufferInUse );

    if ( fTermForceFlush || fAcquiredBufferFill )
    {


        CSXWLatch::ERR errUpgrade = m_sxwlFlushing.ErrUpgradeExclusiveLatchToWriteLatch();
        Assert( errUpgrade != CSXWLatch::ERR::errLatchConflict );
        if ( errUpgrade == CSXWLatch::ERR::errWaitForWriteLatch )
        {
            m_sxwlFlushing.WaitForWriteLatch();
        }

        const INT fPre = AtomicCompareExchange( (LONG*)&(m_rgfBufferState[ipbCurrent]), fBufferInUse, fBufferFlushed );
        Assert( fPre == fBufferInUse );

        INT cOutstandingIO = AtomicIncrement( (LONG*)&m_cOutstandingIO );
        if ( cOutstandingIO > m_cOutstandingIOHighWater )
        {
            m_cOutstandingIOHighWater = cOutstandingIO;
        }


        const BOOL fCleanUpStateSaved = FOSSetCleanupState( fFalse );
        TraceContextScope tcFtl;
        tcFtl->iorReason = *m_piorTraceLog;

        ERR errIO = m_pfapiTraceLog->ErrIOWrite( *tcFtl,
                                                            m_ibWriteBufferCurrent,
                                                            m_cbWriteBufferMax,
                                                            m_rgpbWriteBuffers[ipbCurrent],
                                                            qosIONormal,
                                                            FTLFlushBufferComplete,
                                                            (DWORD_PTR)this );

        (void)FOSSetCleanupState( fCleanUpStateSaved );

        if ( errIO < JET_errSuccess )
        {
            PftlhdrFTLTraceLogHeader()->le_cWriteFailures = PftlhdrFTLTraceLogHeader()->le_cWriteFailures + 1;
        }

        m_ibWriteBufferCurrent += m_cbWriteBufferMax;

        errIO = m_pfapiTraceLog->ErrIOIssue();
        if ( errIO < JET_errSuccess )
        {
            PftlhdrFTLTraceLogHeader()->le_cWriteFailures = PftlhdrFTLTraceLogHeader()->le_cWriteFailures + 1;
        }

        m_sxwlFlushing.ReleaseWriteLatch();
    }
    else
    {
        m_sxwlFlushing.ReleaseSharedLatch();
    }

    return JET_errSuccess;
}


ERR CFastTraceLog::ErrFTLFlushBuffer( __inout void * const pvFlushBufferContext, __in_bcount(cbBuffer) const BYTE * const rgbBuffer, __in const ULONG cbBuffer )
{
    CFastTraceLog * pftl = (CFastTraceLog*)pvFlushBufferContext;

    return pftl->ErrFTLIFlushBuffer( rgbBuffer, cbBuffer, pftl->m_fTerminating );
}

INLINE FTLTDESC CFastTraceLog::FtltdescFTLIGetDescriptor( __in const FTLTID ftltid ) const
{
    Assert( FFTLValidFTLTID( ftltid ) );

    C_ASSERT( _countof(m_ftldesc.m_rgftltdescShortTraceDescriptors) == 128 );

    AssertPREFIX( m_ftldesc.m_cShortTraceDescriptors <= _countof(m_ftldesc.m_rgftltdescShortTraceDescriptors) );
    if ( ftltid < m_ftldesc.m_cShortTraceDescriptors )
    {
        Assert( ftltid < _countof(m_ftldesc.m_rgftltdescShortTraceDescriptors) );
        return m_ftldesc.m_rgftltdescShortTraceDescriptors[ftltid];
    }
    else if ( ( ftltid >= _countof(m_ftldesc.m_rgftltdescShortTraceDescriptors) ) &&
                (INT)( ( ftltid - (USHORT)_countof(m_ftldesc.m_rgftltdescShortTraceDescriptors) ) < m_ftldesc.m_cLongTraceDescriptors ) )
    {
        return m_ftldesc.m_rgftltdescLongTraceDescriptors[ftltid-_countof(m_ftldesc.m_rgftltdescShortTraceDescriptors)];
    }
    return ftltdescDefaultDescriptor;
}

ERR CFastTraceLog::ErrFTLTrace( __in const FTLTID ftltid, __in_bcount(cbTrace) const BYTE * pbTrace, __in const ULONG cbTrace, __in const TICK tickTrace )
{
    ERR err = JET_errSuccess;

    Assert( FFTLValidFTLTID( ftltid ) );

    const FTLTDESC ftltdesc = FtltdescFTLIGetDescriptor( ftltid );

    err = m_ftlb.ErrFTLBTrace( ftltid, ftltdesc, pbTrace, cbTrace, tickTrace );

    return err;
}

ERR CFastTraceLog::ErrFTLIReadBuffer( __out_bcount(cbBuffer) void * pvBuffer, __in QWORD ibOffset, __in ULONG cbBuffer )
{
    ERR err = JET_errSuccess;
    QWORD cbFileSize = 0;

    Call( m_pfapiTraceLog->ErrSize( &cbFileSize, IFileAPI::filesizeLogical ) );

    if ( ibOffset >= cbFileSize )
    {
        Error( ErrERRCheck( JET_errFileIOBeyondEOF ) );
    }
    else
    {
        const ULONG cbReadSize = (ULONG)min( cbBuffer, cbFileSize - ibOffset );
        Assert( ( cbReadSize % OSMemoryPageCommitGranularity() ) == 0 );
        TraceContextScope tcFtl;
        tcFtl->iorReason = *m_piorTraceLog;
        Call( m_pfapiTraceLog->ErrIORead( *tcFtl, ibOffset, cbReadSize, (BYTE*)pvBuffer, qosIONormal ) );

        if ( ( cbReadSize < cbBuffer ) && ( err == JET_errSuccess ) )
        {
            err = ErrERRCheck( JET_wrnBufferTruncated );
        }
    }

HandleError:

    return err;
}

CFastTraceLog::CFTLReader::CFTLReader( CFastTraceLog * pftl, BOOL fKeepStats )
{
    memset( this, 0, sizeof(*this) );

    m_pftl = pftl;

    Expected( m_pftl->m_pfsconfig->CbMaxReadSize() >= m_pftl->m_ftlb.CbFTLBBuffer() );
    m_cbReadBufferSize = rounddn( m_pftl->m_pfsconfig->CbMaxReadSize(), m_pftl->m_ftlb.CbFTLBBuffer() );

    Assert( m_cbReadBufferSize >= m_pftl->m_ftlb.CbFTLBBuffer() );
    Assert( ( m_cbReadBufferSize % m_pftl->m_ftlb.CbFTLBBuffer() ) == 0 );

    m_ibufReadLast = ibufUninitialized;

    m_ibBookmarkNext = 0;
}


CFastTraceLog::CFTLReader::~CFTLReader(  )
{
    for ( ULONG ibuf = 0; ibuf < _countof(m_rgbufReadBuffers); ibuf++ )
    {
        OSMemoryPageFree( m_rgbufReadBuffers[ibuf].pbReadBuffer );
        m_rgbufReadBuffers[ibuf].pbReadBuffer = NULL;
    }
}

ERR CFastTraceLog::CFTLReader::ErrFTLIFillBuffer( __in const QWORD ibBookmarkRead )
{
    ERR err = JET_errSuccess;


    m_ibufReadLast = ( m_ibufReadLast + 1 ) % _countof(m_rgbufReadBuffers);
    AssertPREFIX( m_ibufReadLast >= 0 );
    AssertPREFIX( m_ibufReadLast < _countof(m_rgbufReadBuffers) );


    if ( !m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer )
    {

        Alloc( m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer = (BYTE*)PvOSMemoryPageAlloc( m_cbReadBufferSize, NULL ) );
    }


    memset( m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer, 0, m_cbReadBufferSize );

    m_cReadIOs++;

    err = m_pftl->ErrFTLIReadBuffer( m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer, ibBookmarkRead, m_cbReadBufferSize );
    if ( err == JET_errFileIOBeyondEOF )
    {
        return errNotFound;
    }

    m_rgbufReadBuffers[m_ibufReadLast].ibBookmark = ibBookmarkRead;

HandleError:

    return err;
}

ERR CFastTraceLog::CFTLReader::ErrFTLGetNextTrace( FTLTrace * pftltrace )
{
    ERR err = JET_errSuccess;

    if (  m_ibufReadLast == ibufUninitialized )
    {
        const QWORD ibInitialRead = m_pftl->CbFullHeader();

        Call( ErrFTLIFillBuffer( ibInitialRead ) );

        if ( err == JET_wrnBufferTruncated )
        {
            err = JET_errSuccess;
        }
        
        Assert( m_ibufReadLast != ibufUninitialized );
        AssertPREFIX( m_ibufReadLast >= 0 );
        AssertPREFIX( m_ibufReadLast < _countof(m_rgbufReadBuffers) );
        Assert( m_rgbufReadBuffers[m_ibufReadLast].ibBookmark == ibInitialRead );

        m_ibBookmarkNext = m_rgbufReadBuffers[m_ibufReadLast].ibBookmark + m_pftl->m_ftlb.CbFTLBHeader();
        m_tickBase = *((TICK*)m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer);
    }

    {
        BYTE* pbTrace = m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer + m_ibBookmarkNext - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark;

        pftltrace->ibBookmark = m_ibBookmarkNext;

        if (0 == ((pftltrace->ibBookmark - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark) % m_pftl->m_ftlb.CbFTLBBuffer()) ||
            pbTrace[0] == 0)     {

            while (0 == ((pftltrace->ibBookmark - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark) % m_pftl->m_ftlb.CbFTLBBuffer()) ||
                pbTrace[0] == 0)         {

                Assert((pftltrace->ibBookmark - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark) >= 0);
                Assert(m_pftl->m_ftlb.CbFTLBHeader() <= m_cbReadBufferSize);
                Assert(m_pftl->m_ftlb.CbFTLBBuffer() <= m_cbReadBufferSize);
                Expected(m_pftl->m_ftlb.CbFTLBBuffer() < m_cbReadBufferSize);

                Assert((INT)(pftltrace->ibBookmark - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark) <= m_cbReadBufferSize);
                Assert((pftltrace->ibBookmark - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark) <= (QWORD)m_cbReadBufferSize);


                const INT cbBlankFTLBufferLeft = (INT)(roundup(pftltrace->ibBookmark, (QWORD)m_pftl->m_ftlb.CbFTLBBuffer()) - pftltrace->ibBookmark);
                Assert((QWORD)cbBlankFTLBufferLeft == (roundup(pftltrace->ibBookmark, (QWORD)m_pftl->m_ftlb.CbFTLBBuffer()) - pftltrace->ibBookmark));

                Assert(cbBlankFTLBufferLeft >= 0);
                Assert(pftltrace->ibBookmark + cbBlankFTLBufferLeft == roundup(pftltrace->ibBookmark, m_pftl->m_ftlb.CbFTLBBuffer()));

                if (cbBlankFTLBufferLeft >= (INT)(m_pftl->m_ftlb.CbFTLBBuffer() - m_pftl->m_ftlb.CbFTLBHeader()))             {
                    m_cBlankBuffers++;
                    m_cbBlankEmpty += m_pftl->m_ftlb.CbFTLBBuffer();
                }
                else if (cbBlankFTLBufferLeft == 0)             {
                    m_cFullBuffers++;
                }
                else             {
                    Assert(cbBlankFTLBufferLeft);
                    m_cPartialBuffers++;
                    m_cbPartialEmpty += cbBlankFTLBufferLeft;
                }

                const QWORD ibNextFTLBuffer = pftltrace->ibBookmark + cbBlankFTLBufferLeft;


                if (!FBounded(ibNextFTLBuffer, m_rgbufReadBuffers[m_ibufReadLast].ibBookmark, m_cbReadBufferSize))             {
                    const INT ibufPrev = m_ibufReadLast;
                    Call(ErrFTLIFillBuffer(ibNextFTLBuffer));

                    if (err == JET_wrnBufferTruncated)                 {
                        err = JET_errSuccess;
                    }

                    Assert(ibufPrev != m_ibufReadLast);
                    Assert(m_rgbufReadBuffers[m_ibufReadLast].ibBookmark == ibNextFTLBuffer);
                }

                m_tickBase = *((TICK*)(m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer + ibNextFTLBuffer - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark));
                m_ibBookmarkNext = ibNextFTLBuffer + m_pftl->m_ftlb.CbFTLBHeader();

                pbTrace = m_rgbufReadBuffers[m_ibufReadLast].pbReadBuffer + m_ibBookmarkNext - m_rgbufReadBuffers[m_ibufReadLast].ibBookmark;
                pftltrace->ibBookmark = m_ibBookmarkNext;
            }
        }


        err = CFastTraceLogBuffer::ErrFTLBParseTraceHeader(pbTrace,
            &(pftltrace->ftltid),
            m_tickBase,
            &(pftltrace->tick));

        if (err == errNotFound)     {
            return err;
        }

        Call(err);

        {
            const FTLTDESC ftltdesc = m_pftl->FtltdescFTLIGetDescriptor(pftltrace->ftltid);


            Call(CFastTraceLogBuffer::ErrFTLBParseTraceData(pbTrace,
                pftltrace->ftltid,
                ftltdesc,
                &(pftltrace->cbTraceData),
                &(pftltrace->pbTraceData)));

            m_ibBookmarkNext = pftltrace->ibBookmark + (pftltrace->pbTraceData - pbTrace) + pftltrace->cbTraceData;

            #ifdef DEBUG
            m_ftltracePrevPrev = m_ftltracePrev;
            m_ibufTracePrevPrev = m_ibufTracePrev;
            m_ftltracePrev = *pftltrace;
            m_ibufTracePrev = m_ibufReadLast;
            #endif
        }
    }
HandleError:


    return err;
}



const WCHAR * WszTraceLogState( const FASTTRACELOGSTATE tracelogstate )
{
    return ( ( tracelogstate == eFTLCreated ) ?
                L"eFTLCreated" :
                ( tracelogstate == eFTLDirty ) ?
                    L"eFTLDirty" :
                    ( tracelogstate == eFTLClean ) ?
                        L"eFTLClean" :
                        ( tracelogstate == eFTLPostProcessed ) ?
                            L"eFTLPostProcessed" :
                            L"UNKNOWN!" );
}

ERR ErrFTLFormatFileTimeAsDateTime( __int64 time, __out_bcount_z(cbDate) PWSTR const pwszDate, const size_t cbDate, __out_bcount_z(cbTime) PWSTR const pwszTime, const size_t cbTime )
{
    ERR err = JET_errSuccess;
    size_t  cbCrud;

    Expected( pwszDate );
    Expected( cbDate >= sizeof( WCHAR ) );
    Expected( pwszTime );
    Expected( cbTime >= sizeof( WCHAR ) );

    if ( ( NULL == pwszDate ) || ( cbDate < sizeof( WCHAR ) ) )
    {
        return ErrERRCheck( JET_errInvalidParameter );
    }

    if ( ( NULL == pwszTime ) || ( cbTime < sizeof( WCHAR ) ) )
    {
        return ErrERRCheck( JET_errInvalidParameter );
    }

    pwszDate[0] = L'\0';
    pwszTime[0] = L'\0';

    if ( time == 0 )
    {
        OSStrCbCopyW( pwszDate, cbDate, L"Unset(N/A)" );
        Assert( pwszTime[0] == L'\0' );
        return JET_errSuccess;
    }
    
    CallR( ErrUtilFormatFileTimeAsDate( time, pwszDate, cbDate/sizeof(pwszDate[0]), &cbCrud ) );
    CallR( ErrUtilFormatFileTimeAsTimeWithSeconds( time, pwszTime, cbTime/sizeof(pwszTime[0]), &cbCrud ) );

    return err;
}

ERR CFastTraceLog::ErrFTLDumpHeader()
{
    ERR err = JET_errSuccess;
    WCHAR wszDate[60];
    WCHAR wszTime[60];

    Assert( m_pfapiTraceLog );

    wprintf( L"\n" );
    wprintf( L"FTL Trace File Header:\n" );
    wprintf( L"\n" );

    wprintf( L"            le_ulChecksum: 0x%x\n", (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulChecksumReserved) );

    wprintf( L"         le_ulFTLVersion*: %d.%02d.%02d (0x%x.0x%x.0x%x)\n",
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMajor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMinor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulFTLVersionUpdate),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMajor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulFTLVersionMinor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulFTLVersionUpdate) );
    wprintf( L"         le_tracelogstate: %ws (%d 0x%x)\n",
                            WszTraceLogState( (FASTTRACELOGSTATE)(ULONG)PftlhdrFTLTraceLogHeader()->le_tracelogstate ),
                            (ULONG)(PftlhdrFTLTraceLogHeader()->le_tracelogstate), (ULONG)(PftlhdrFTLTraceLogHeader()->le_tracelogstate) );

    wprintf( L"            le_ulSchemaID: %d (0x%x)\n", (ULONG)(PftlhdrFTLTraceLogHeader()->le_ulSchemaID), (ULONG)(PftlhdrFTLTraceLogHeader()->le_ulSchemaID) );
    wprintf( L"      le_ulSchemaVersion*: %d.%02d.%02d (0x%x.0x%x.0x%x)\n",
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMajor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMinor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionUpdate),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMajor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionMinor),
                            (ULONG) (PftlhdrFTLTraceLogHeader()->le_ulSchemaVersionUpdate) );

    wprintf( L"              le_cReOpens: %d\n", (ULONG)(PftlhdrFTLTraceLogHeader()->le_cReOpens) );
    wprintf( L"           le_cRecoveries: %d\n", (ULONG)(PftlhdrFTLTraceLogHeader()->le_cRecoveries) );
    wprintf( L"     le_cbLastKnownBuffer: %I64d\n", (QWORD)(PftlhdrFTLTraceLogHeader()->le_cbLastKnownBuffer) );
    wprintf( L"        le_cWriteFailures: %I64d\n", (QWORD)(PftlhdrFTLTraceLogHeader()->le_cWriteFailures) );

    Call( ErrFTLFormatFileTimeAsDateTime( PftlhdrFTLTraceLogHeader()->le_ftFirstOpen, wszDate, sizeof(wszDate), wszTime, sizeof(wszTime) ) );
    wprintf( L"           le_ftFirstOpen: %ws %ws\n", wszDate, wszTime );

    Call( ErrFTLFormatFileTimeAsDateTime( PftlhdrFTLTraceLogHeader()->le_ftLastOpen, wszDate, sizeof(wszDate), wszTime, sizeof(wszTime) ) );
    wprintf( L"            le_ftLastOpen: %ws %ws\n", wszDate, wszTime );
    Call( ErrFTLFormatFileTimeAsDateTime( PftlhdrFTLTraceLogHeader()->le_ftLastClose, wszDate, sizeof(wszDate), wszTime, sizeof(wszTime) ) );
    wprintf( L"           le_ftLastClose: %ws %ws\n", wszDate, wszTime );

    wprintf( L"          le_cMaxWriteIOs: %d\n", (ULONG)(PftlhdrFTLTraceLogHeader()->le_cMaxWriteIOs) );
    wprintf( L"      le_cMaxWriteBuffers: %d\n", (ULONG)(PftlhdrFTLTraceLogHeader()->le_cMaxWriteBuffers) );

    Call( ErrFTLFormatFileTimeAsDateTime( PftlhdrFTLTraceLogHeader()->le_ftPostProcessed, wszDate, sizeof(wszDate), wszTime, sizeof(wszTime) ) );
    wprintf( L"       le_ftPostProcessed: %ws %ws\n", wszDate, wszTime );

HandleError:

    if ( err < JET_errSuccess )
    {
        wprintf( L" Error Interpriting time field %d\n", err );
    }

    return err;
}

ERR CFastTraceLog::CFTLReader::ErrFTLDumpStats()
{

    wprintf( L"\n" );
    wprintf( L"FTL Stats::Usage Stats\n" );
    wprintf( L"\n" );

    wprintf( L"    m_cReadIOs: %I64d\n", m_cReadIOs );

    wprintf( L"\n" );
    wprintf( L"FTL Stats::Trace Stats\n" );
    wprintf( L"\n" );
    
    wprintf( L"    m_cFullBuffers: %I64d\n", m_cFullBuffers );
    wprintf( L"    m_cPartialBuffers: %I64d\n", m_cPartialBuffers );
    wprintf( L"    m_cbPartialEmpty: %I64d (%2.2f ave / partial buffer)\n", m_cbPartialEmpty, ( (float)m_cbPartialEmpty / (float)m_cPartialBuffers ) );
    wprintf( L"    m_cBlankBuffers: %I64d\n", m_cBlankBuffers );
    wprintf( L"    m_cbBlankEmpty: %I64d\n", m_cbBlankEmpty );
    
    return JET_errSuccess;
}


const USER_CONTEXT_DESC UserTraceContext::s_ucdNil = { 0 };

ERR USER_CONTEXT_DESC::ErrCopyDescString( char* const szDest, const char* const szSrc, const INT cbSrc )
{
    if ( cbSrc < USER_CONTEXT_DESC::cbStrMax )
    {
        memcpy( szDest, szSrc, cbSrc );
        INT cchSrc = cbSrc / sizeof( char );
        szDest[ cchSrc ] = 0;
        return JET_errSuccess;
    }
    else
    {
        const INT cchMax = USER_CONTEXT_DESC::cchStrMax - 4;
        const INT cbMax = cchMax * sizeof( char );
        memcpy( szDest, szSrc, cbMax );
        szDest[ cchMax ] = szDest[ cchMax + 1 ] = szDest[ cchMax + 2 ] = '.';
        szDest[ cchMax + 3 ] = 0;
        return ErrERRCheck( JET_wrnBufferTruncated );
    }
}

UserTraceContext::UserTraceContext( const UserTraceContext& rhs, bool fCopyUserContextDesc )
{
    context = rhs.context;
    dwCorrelationID = rhs.dwCorrelationID;
    dwIOSessTraceFlags = rhs.dwIOSessTraceFlags;
    pUserContextDesc = const_cast<USER_CONTEXT_DESC*>( &s_ucdNil );

    if ( fCopyUserContextDesc && rhs.FUserContextDescInitialized() )
    {
        const BOOL fCleanUpStateSaved = FOSSetCleanupState( fFalse );

        ERR err = ErrLazyInitUserContextDesc();
        FOSSetCleanupState( fCleanUpStateSaved );

        if ( err >= JET_errSuccess )
        {
            memcpy( pUserContextDesc, rhs.pUserContextDesc, sizeof( USER_CONTEXT_DESC ) );
        }
    }
}

ERR UserTraceContext::ErrLazyInitUserContextDesc()
{
    ERR err = JET_errSuccess;
    if ( !FUserContextDescInitialized() )
    {
        USER_CONTEXT_DESC* pNew;
        Alloc( pNew = new USER_CONTEXT_DESC() );
        pUserContextDesc = pNew;
    }

HandleError:
    return err;
}

void UserTraceContext::DeepCopy( const UserTraceContext& utcFrom )
{
    if ( utcFrom.FUserContextDescInitialized() )
    {
        const BOOL fCleanUpStateSaved = FOSSetCleanupState( fFalse );
        
        ERR err = ErrLazyInitUserContextDesc();
        FOSSetCleanupState( fCleanUpStateSaved );

        if ( err >= JET_errSuccess )
        {
            memcpy( pUserContextDesc, utcFrom.pUserContextDesc, sizeof( USER_CONTEXT_DESC ) );
        }
    }
    else
    {
        if ( FUserContextDescInitialized() )
        {
            memcpy( pUserContextDesc, &s_ucdNil, sizeof( USER_CONTEXT_DESC ) );
        }
    }

    if ( (BOOL) UlConfigOverrideInjection( 48010, fFalse ) )
    {
        Assert( utcFrom.context.dwUserID != 0 );
    }

    context = utcFrom.context;
    dwCorrelationID = utcFrom.dwCorrelationID;
    dwIOSessTraceFlags = utcFrom.dwIOSessTraceFlags;
}

const UserTraceContext* TLSSetUserTraceContext( const UserTraceContext* putc )
{
    OSTLS* postls = Postls();
    const UserTraceContext* ptcPrev = postls->putc;
    postls->putc = putc;
    return ptcPrev;
}

const TraceContext* PetcTLSGetEngineContext()
{
    OSTLS* postls = Postls();
    return &postls->etc;
}

const TraceContext* PetcTLSGetEngineContextCached( TLS* ptlsCached )
{
    OSTLS* postls = PostlsFromTLS( ptlsCached );
    Assert( postls == Postls() );
    return &postls->etc;
}

const UserTraceContext* PutcTLSGetUserContext()
{
    OSTLS* postls = Postls();
    return postls->putc;
}

GetCurrUserTraceContext::GetCurrUserTraceContext() :
    m_putcTls( Postls()->putc ),
    m_utcSysDefault( OCUSER_SYSTEM )
{
    if ( (BOOL) UlConfigOverrideInjection( 48010, fFalse ) )
    {
        Assert( m_putcTls == NULL || m_putcTls->context.dwUserID != 0 );
    }
}

ULONG OpTraceContext()
{
    return FOSDllUp() ? Postls()->etc.iorReason.Ioru() : 0;
}

#endif

