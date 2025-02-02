/*	$OpenBSD: vis.h,v 1.15 2015/07/20 01:52:27 millert Exp $	*/
/*	$NetBSD: vis.h,v 1.4 1994/10/26 00:56:41 cgd Exp $	*/

/*-
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)vis.h	5.9 (Berkeley) 4/3/91
 */

/// use octal \ddd format
pub const VIS_OCTAL: i32 = 0x01;
/// use \[nrft0..] where appropriate
pub const VIS_CSTYLE: i32 = 0x02;

/// also encode space
pub const VIS_SP: i32 = 0x04;
/// also encode tab
pub const VIS_TAB: i32 = 0x08;
/// also encode newline
pub const VIS_NL: i32 = 0x10;
pub const VIS_WHITE: i32 = VIS_SP | VIS_TAB | VIS_NL;
/// only encode "unsafe" characters
pub const VIS_SAFE: i32 = 0x20;
/// backslash-escape double quotes
pub const VIS_DQ: i32 = 0x200;
/// encode all characters
pub const VIS_ALL: i32 = 0x400;

/// inhibit printing '\'
pub const VIS_NOSLASH: i32 = 0x40;
/// encode glob(3) magics and '#'
pub const VIS_GLOB: i32 = 0x100;

/// character valid
pub const UNVIS_VALID: i32 = 1;
/// character valid, push back passed char
pub const UNVIS_VALIDPUSH: i32 = 2;
/// valid sequence, no character produced
pub const UNVIS_NOCHAR: i32 = 3;
/// unrecognized escape sequence
pub const UNVIS_SYNBAD: i32 = -1;
/// decoder in unknown state (unrecoverable)
pub const UNVIS_ERROR: i32 = -2;

/// no more characters
pub const UNVIS_END: i32 = 1;
