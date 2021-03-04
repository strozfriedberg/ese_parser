// decompress.cpp
//

#undef  UNICODE
#undef  _UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <tchar.h>
#include <strsafe.h>

#include <iostream>

#include <Esent.h>

#define ESENT                                           1
#define fInvalidUsage                                   0
#define JET_cbKeyMost_OLD                               256
#define JET_resoperSize                                 0
#define JET_IOPriorityLowForCheckpoint                  0
#define JET_IOPriorityLowForScavenge                    0
#define JET_efvExtHdrRootFieldAutoIncStorageReleased    1
#define JET_efvLogtimeGenMaxRequired                    5
#define JET_dbstateIncrementalReseedInProgress          -2
#define JET_dbstateRevertInProgress                     -3
#define JET_dbstateDirtyAndPatchedShutdown              -4

#define COLLAssertTrack( X )
#define MSINTERNAL

class INST;
using JET_RESID                 = int;
using JET_RESOPER               = int;
using JET_RECSIZE3              = void;
using TICK                      = int;
using JET_ENGINEFORMATVERSION   = long;
using JET_DBINFOMISC7           = void;
using JET_PFNINITCALLBACK       = void *;
using JET_THREADSTATS4          = void *;
using HRT                       = void *;

BOOL FOSDllUp();
BOOL FOSLayerUp();
TICK TickOSTimeCurrent();

#include "ms/cc.hxx"
#include "ms/types.hxx"
#include "ms/memory.hxx"
#include "ms/error.hxx"
#include "ms/math.hxx"
#include "ms/thread.hxx"
#include "ms/sync.hxx"
#include "ms/syncu.hxx"
#include "ms/task.hxx"
#include "ms/cprintf.hxx"
#include "ms/trace.hxx"
#include "ms/oseventtrace.hxx"
#include "ms/sysinfo.hxx"
#include "ms/norm.hxx"
#include "ms/privconsts.h"
#include "ms/cresmgr.hxx"
#include "ms/string.hxx"
#include "ms/_jet.hxx"
#include "ms/stat.hxx"
#include "ms/daedef.hxx"


struct CDataCompressor {
    enum COMPRESSION_SCHEME
    {
        COMPRESS_NONE,
        COMPRESS_7BITASCII = 0x1,
        COMPRESS_7BITUNICODE = 0x2,
        COMPRESS_XPRESS = 0x3,
        COMPRESS_SCRUB = 0x4,
        COMPRESS_XPRESS9 = 0x5,
        COMPRESS_XPRESS10 = 0x6,
        COMPRESS_MAXIMUM = 0x1f,
    };

    ERR ErrDecompress(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual );
    ERR ErrDecompress7BitAscii_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual );
    ERR ErrDecompress7BitUnicode_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual );
    ERR ErrDecompressXpress_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, *pcbDataActual ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual);
    ERR ErrDecompressScrub_(
        const DATA& data );
    #ifdef XPRESS9_COMPRESSION
    ERR ErrDecompressXpress9_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual,
        IDataCompressorStats * const pstats );
    #endif
    #ifdef XPRESS10_COMPRESSION
    ERR ErrDecompressXpress10_(
        const DATA& dataCompressed,
        _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
        const INT cbDataMax,
        _Out_ INT * const pcbDataActual,
        IDataCompressorStats * const pstats,
        const BOOL fForceSoftwareDecompression,
        BOOL * pfUsedCorsica );
    #endif
};

ERR CDataCompressor::ErrDecompress(
    const DATA& dataCompressed,
    _Out_writes_bytes_to_opt_( cbDataMax, min( cbDataMax, *pcbDataActual ) ) BYTE * const pbData,
    const INT cbDataMax,
    _Out_ INT * const pcbDataActual )
{
    ERR err = JET_errSuccess;
    BOOL fUnused = fFalse;

    const BYTE bHeader = *(BYTE *)dataCompressed.Pv();
    const BYTE bIdentifier = bHeader >> 3;

    switch( bIdentifier )
    {
        case COMPRESS_7BITASCII:
            Call( ErrDecompress7BitAscii_( dataCompressed, pbData, cbDataMax, pcbDataActual ) );
            break;
        case COMPRESS_7BITUNICODE:
            Call( ErrDecompress7BitUnicode_( dataCompressed, pbData, cbDataMax, pcbDataActual ) );
            break;
        case COMPRESS_XPRESS:
            Call( ErrDecompressXpress_( dataCompressed, pbData, cbDataMax, pcbDataActual) );
            break;
        case COMPRESS_SCRUB:
            Expected( pbData == NULL );
            Expected( cbDataMax == 0 );
            Assert( g_fRepair || FNegTest( fInvalidUsage ) );
            *pcbDataActual = 0;
            Call( ErrDecompressScrub_( dataCompressed ) );
            break;
            #ifdef XPRESS9_COMPRESSION
        case COMPRESS_XPRESS9:
            Call( ErrDecompressXpress9_( dataCompressed, pbData, cbDataMax, pcbDataActual, pstats ) );
            break;
            #endif
            #ifdef XPRESS10_COMPRESSION
        case COMPRESS_XPRESS10:
            Call( ErrDecompressXpress10_( dataCompressed, pbData, cbDataMax, pcbDataActual, pstats, fFalse, &fUnused ) );
            break;
            #endif
        default:
            *pcbDataActual = 0;
            Call( ErrERRCheck( JET_errDecompressionFailed ) );
            break;
    }

    if( *pcbDataActual > cbDataMax )
    {
        Assert( JET_wrnBufferTruncated == err );
    }

HandleError:
    Expected( ( bIdentifier != COMPRESS_SCRUB ) || ( err == wrnRECCompressionScrubDetected ) );

    return err;
}


int main()
{
    std::cout << "Hello World!\n";
}

