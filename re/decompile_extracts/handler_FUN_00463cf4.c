// ==== FUN_00463cf4 @ 00463cf4 ====

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined8 FUN_00463cf4(longlong param_1,undefined8 param_2,wchar_t *param_3,ulonglong param_4)

{
  int iVar1;
  short sVar2;
  short sVar3;
  ulonglong uVar4;
  wchar_t *pwVar5;
  undefined8 *puVar6;
  byte bVar7;
  int iVar8;
  ulonglong *puVar9;
  HWND pHVar10;
  LPCWSTR pWVar11;
  undefined8 uVar12;
  longlong *plVar13;
  undefined7 extraout_var;
  longlong *plVar14;
  char *pcVar15;
  wchar_t *pwVar16;
  longlong lVar17;
  undefined8 *puVar18;
  short sVar19;
  int *piVar20;
  short *psVar21;
  longlong lVar22;
  char **ppcVar23;
  undefined8 *local_1798;
  LPWSTR local_1790;
  short *local_1788;
  undefined *local_1780;
  longlong local_1778;
  undefined8 *local_1770;
  undefined *local_1768;
  undefined *local_1760;
  undefined *local_1758;
  undefined *local_1750;
  wchar_t *local_1748;
  longlong local_1740;
  undefined8 *local_1738;
  longlong local_1730;
  ulonglong local_1728;
  undefined8 local_1720;
  short *local_1718;
  ulonglong local_1710;
  longlong local_1708;
  undefined8 *local_1700;
  longlong local_16f8;
  LPCWSTR local_16f0;
  undefined8 local_16e8;
  undefined8 local_16e0;
  undefined8 *local_16d8;
  LPCWSTR local_16d0;
  undefined8 *local_16c8;
  ulonglong local_16c0;
  ulonglong local_16b8;
  short *local_16b0;
  longlong local_16a8;
  longlong local_16a0;
  char *local_1698 [5];
  char *local_1670;
  char *local_1668;
  char *local_1660;
  char *local_1658;
  char *local_1650;
  char *local_1648;
  char *local_1640;
  char *local_1638;
  char *local_1630;
  char *local_1628;
  char *local_1620;
  char *local_1618;
  char *local_1610;
  char *local_1608;
  char *local_1600;
  char *local_15f8;
  char *local_15f0;
  char *local_15e8;
  char *local_15e0;
  char *local_15d8;
  char *local_15d0;
  ulonglong local_15c8;
  longlong local_15c0;
  longlong local_15b8;
  longlong local_15b0;
  longlong local_15a8;
  longlong local_15a0;
  ulonglong local_1598;
  longlong local_1590;
  tagMENUITEMINFOW local_1588;
  undefined8 local_1538;
  undefined8 local_1530;
  undefined8 local_1508;
  HWND local_1500;
  undefined4 local_1498;
  undefined8 local_b18;
  HWND local_b10;
  undefined4 local_aa8;
  char local_128;
  undefined1 local_127;
  undefined1 local_126;
  undefined1 local_125;
  undefined1 local_124;
  undefined1 local_123;
  undefined1 local_122;
  undefined1 local_121;
  undefined1 local_120;
  undefined1 local_11f;
  undefined1 local_11e;
  undefined1 local_11d;
  undefined1 local_11c;
  undefined1 local_11b;
  undefined1 local_11a;
  undefined1 local_119;
  undefined1 local_118;
  undefined1 local_117;
  undefined1 local_116;
  undefined1 local_115;
  undefined1 local_114;
  undefined1 local_113;
  undefined1 local_112;
  undefined1 local_111;
  undefined1 local_110;
  undefined1 local_10f;
  char local_108;
  undefined1 local_107;
  undefined1 local_106;
  undefined1 local_105;
  undefined1 local_104;
  undefined1 local_103;
  undefined1 local_102;
  undefined1 local_101;
  undefined1 local_100;
  undefined1 local_ff;
  undefined1 local_fe;
  undefined1 local_fd;
  undefined1 local_fc;
  undefined1 local_fb;
  undefined1 local_fa;
  undefined1 local_f9;
  undefined1 local_f8;
  undefined1 local_f7;
  undefined1 local_f6;
  undefined1 local_f5;
  undefined1 local_f4;
  undefined1 local_f3;
  undefined1 local_f2;
  undefined1 local_f1;
  undefined1 local_f0;
  undefined1 local_ef;
  undefined1 local_ee;
  WCHAR local_e8 [80];
  undefined8 local_48;
  undefined8 uStack_40;
  
  uStack_40 = 0x463d09;
  local_48 = DAT_00547f48;
  sVar19 = (short)param_3;
  FUN_0040331c((longlong *)&local_1798,"");
  if (sVar19 == -0x1ec0) {
    puVar9 = FUN_004a3f04(&local_16b8,0x3a,param_3,param_4);
    param_4 = 0;
    local_15c8 = *puVar9;
    param_3 = (wchar_t *)0x0;
    FUN_004b2494(&local_b18,local_15c8,0,(HPROPSHEETPAGE)0x0);
    piVar20 = (int *)(local_16b8 - 0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_16b8 - 0xc));
      }
    }
    if (local_b10 == (HWND)0x0) {
      local_aa8 = 7;
    }
    else {
      param_4 = 0;
      param_3 = (wchar_t *)0x7;
      SendMessageW(local_b10,0x465,7,0);
    }
    pHVar10 = GetActiveWindow();
    FUN_004613b0((longlong)&local_b18,pHVar10);
    FUN_004ae908(&local_b18);
  }
  iVar8 = DAT_00546c60;
  if (sVar19 == -0x4dad) {
    puVar9 = FUN_004a3f04(&local_1728,0x3a,param_3,param_4);
    param_4 = 0;
    local_1598 = *puVar9;
    param_3 = (wchar_t *)0x0;
    FUN_004b2494(&local_1508,local_1598,0,(HPROPSHEETPAGE)0x0);
    piVar20 = (int *)(local_1728 - 0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_1728 - 0xc));
      }
    }
    if (local_1500 == (HWND)0x0) {
      local_1498 = 4;
    }
    else {
      param_4 = 0;
      param_3 = (wchar_t *)0x4;
      SendMessageW(local_1500,0x465,4,0);
    }
    pHVar10 = GetActiveWindow();
    FUN_004613b0((longlong)&local_1508,pHVar10);
    FUN_004ae908(&local_1508);
  }
  else if (sVar19 == -0x7ff2) {
    local_128 = 'h';
    local_127 = 0x74;
    local_126 = 0x74;
    local_125 = 0x70;
    local_124 = 0x3a;
    local_123 = 0x2f;
    local_122 = 0x2f;
    local_121 = 0x77;
    local_120 = 0x77;
    local_11f = 0x77;
    local_11e = 0x2e;
    local_11d = 0x73;
    local_11c = 0x6f;
    local_11b = 0x66;
    local_11a = 0x74;
    local_119 = 0x77;
    local_118 = 0x61;
    local_117 = 0x72;
    local_116 = 0x65;
    local_115 = 0x6f;
    local_114 = 0x6b;
    local_113 = 0x2e;
    local_112 = 100;
    local_111 = 0x65;
    local_110 = 0x2f;
    local_10f = 0;
    local_108 = 'h';
    local_107 = 0x74;
    local_106 = 0x74;
    local_105 = 0x70;
    local_104 = 0x3a;
    local_103 = 0x2f;
    local_102 = 0x2f;
    local_101 = 0x77;
    local_100 = 0x77;
    local_ff = 0x77;
    local_fe = 0x2e;
    local_fd = 0x73;
    local_fc = 0x6f;
    local_fb = 0x66;
    local_fa = 0x74;
    local_f9 = 0x77;
    local_f8 = 0x61;
    local_f7 = 0x72;
    local_f6 = 0x65;
    local_f5 = 0x6f;
    local_f4 = 0x6b;
    local_f3 = 0x2e;
    local_f2 = 99;
    local_f1 = 0x6f;
    local_f0 = 0x6d;
    local_ef = 0x2f;
    local_ee = 0;
    pWVar11 = (LPCWSTR)FUN_00402340(&DAT_00554358,0);
    pcVar15 = &local_128;
    if (DAT_00546c60 == 0) {
      pcVar15 = &local_108;
    }
    FUN_00439680(pcVar15,pWVar11);
  }
  else if (sVar19 == -0x7ff3) {
    iVar8 = lstrlenW(L"?Kontakt");
    lVar22 = (longlong)iVar8;
    uVar12 = FUN_004020b8((longlong *)&local_1798,iVar8);
    if ((int)uVar12 != 0) {
      pwVar16 = L"?Kontakt";
      goto LAB_004640d2;
    }
  }
  else if (sVar19 == -0x4c52) {
    FUN_00460c74(param_1);
  }
  else if (sVar19 == -0x7f71) {
    plVar13 = FUN_0040329c((longlong *)&local_16b0,(longlong *)&DAT_00554108);
    lVar22 = *plVar13;
    local_1768 = PTR_DAT_00546000;
    iVar8 = lstrlenW(L"?seite=paypal&Program=");
    param_4 = (ulonglong)*(uint *)(lVar22 + -8);
    param_3 = L"?seite=paypal&Program=";
    FUN_00402204((longlong *)&local_1768,iVar8,(undefined8 *)L"?seite=paypal&Program=",
                 *(uint *)(lVar22 + -8),(undefined8 *)*plVar13);
    FUN_00402154((longlong *)&local_1798,(longlong *)&local_1768);
    piVar20 = (int *)(local_1768 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
      }
    }
    piVar20 = (int *)(local_16b0 + -6);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
joined_r0x004648c8:
      if (iVar8 + -1 < 1) {
        pwVar16 = local_16b0 + -6;
LAB_004648d3:
        thunk_FUN_0050e3a0(pwVar16);
      }
    }
  }
  else if (sVar19 == -0x7ff4) {
    iVar8 = lstrlenW(L"?Freeware");
    lVar22 = (longlong)iVar8;
    uVar12 = FUN_004020b8((longlong *)&local_1798,iVar8);
    if ((int)uVar12 != 0) {
      pwVar16 = L"?Freeware";
LAB_004640d2:
      puVar18 = local_1798;
      pwVar5 = (wchar_t *)(lVar22 * 2);
      param_3 = pwVar5;
      FUN_0050dc70(local_1798,(undefined8 *)pwVar16,(ulonglong)pwVar5);
      *(int *)(puVar18 + -1) = (int)lVar22;
      *(undefined2 *)((longlong)pwVar5 + (longlong)puVar18) = 0;
    }
  }
  else if (sVar19 == -0x7ff1) {
    puVar9 = FUN_004a3f04((ulonglong *)&local_1718,0xd,param_3,param_4);
    uVar4 = *puVar9;
    local_1758 = PTR_DAT_00546000;
    iVar8 = lstrlenW(L"?Kontakt&Betreff=");
    param_4 = (ulonglong)*(uint *)(uVar4 - 8);
    param_3 = L"?Kontakt&Betreff=";
    FUN_00402204((longlong *)&local_1758,iVar8,(undefined8 *)L"?Kontakt&Betreff=",
                 *(uint *)(uVar4 - 8),(undefined8 *)*puVar9);
    FUN_00402154((longlong *)&local_1798,(longlong *)&local_1758);
    piVar20 = (int *)(local_1758 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
      }
    }
    piVar20 = (int *)(local_1718 + -6);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      local_16b0 = local_1718;
      goto joined_r0x004648c8;
    }
  }
  else if (sVar19 == -0x7ad4) {
    iVar8 = lstrlenW(L"?seite=faq-Q-DIR&faq=0");
    lVar22 = (longlong)iVar8;
    uVar12 = FUN_004020b8((longlong *)&local_1798,iVar8);
    if ((int)uVar12 != 0) {
      pwVar16 = L"?seite=faq-Q-DIR&faq=0";
      goto LAB_004640d2;
    }
  }
  else if (sVar19 == -0x7a6d) {
    FUN_0040331c((longlong *)&local_1748,"?page=Windows/Info/");
    pwVar16 = local_1748;
    param_4 = (ulonglong)*(uint *)(DAT_00554108 + -1);
    local_1780 = PTR_DAT_00546000;
    param_3 = local_1748;
    FUN_00402204((longlong *)&local_1780,*(int *)(local_1748 + -4),(undefined8 *)local_1748,
                 *(uint *)(DAT_00554108 + -1),DAT_00554108);
    FUN_00402154((longlong *)&local_1798,(longlong *)&local_1780);
    piVar20 = (int *)(local_1780 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      pwVar16 = local_1748;
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
        pwVar16 = local_1748;
      }
    }
    pwVar16 = pwVar16 + -6;
    if (pwVar16 != (wchar_t *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *(int *)pwVar16;
      *(int *)pwVar16 = *(int *)pwVar16 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) goto LAB_004648d3;
    }
  }
  else if (sVar19 == -0x1eba) {
    plVar13 = FUN_0040331c(&local_15a0,"");
    FUN_0049fd78(plVar13);
  }
  else if (sVar19 == -0x1eb9) {
    local_1698[0] = "German";
    local_1698[1] = "de-DE";
    local_1698[2] = "English";
    local_1698[3] = "en-US";
    local_1698[4] = "Polish";
    local_1670 = "pl-PL";
    local_1668 = "Traditional-Chinese";
    local_1660 = "zh-TW";
    local_1658 = "Italian";
    local_1650 = "it-IT";
    local_1648 = "French";
    local_1640 = "fr-FR";
    local_1638 = "Simplified-Chinese";
    local_1630 = "zh-CN";
    local_1628 = "Russian";
    local_1620 = "ru-RU";
    local_1618 = "Korean";
    local_1610 = "ko-KR";
    local_1608 = "Japanese";
    local_1600 = "ja-JP";
    local_15f8 = "Brazil";
    local_15f0 = "pt-BR";
    local_15e8 = "Spanish";
    local_15e0 = "es-ES";
    local_15d8 = "Arabic";
    local_15d0 = "ar-SA";
    FUN_0050daf0((ulonglong *)&local_1588,0,0x48);
    local_1588.dwTypeData = local_e8;
    local_1588.cbSize = 0x48;
    local_1588.fMask = 0x17;
    local_1588.fType = 0;
    local_1588.cch = 0x50;
    GetMenuItemInfoW(*(HMENU *)(param_1 + 0xd18),iVar8 + 0xab7c,0,&local_1588);
    local_1788 = (short *)PTR_DAT_00546000;
    FUN_004f75dc((longlong *)&local_1788,local_e8,0,L'\t');
    FUN_0040331c((longlong *)&local_1790,"en-us");
    iVar8 = 0;
    lVar22 = 0;
    ppcVar23 = local_1698;
    do {
      FUN_0040331c(&local_1778,*ppcVar23);
      lVar17 = local_1778;
      psVar21 = local_1788;
      do {
        sVar2 = *psVar21;
        sVar3 = *(short *)((longlong)psVar21 + (local_1778 - (longlong)local_1788));
        if (sVar2 != sVar3) break;
        psVar21 = psVar21 + 1;
      } while (sVar3 != 0);
      if (sVar2 == sVar3) {
        plVar13 = FUN_0040331c(&local_16a0,local_1698[(longlong)iVar8 * 2 + 1]);
        FUN_00402154((longlong *)&local_1790,plVar13);
        piVar20 = (int *)(local_16a0 + -0xc);
        if (piVar20 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar8 = *piVar20;
          *piVar20 = *piVar20 + -1;
          UNLOCK();
          lVar17 = local_1778;
          if (iVar8 + -1 < 1) {
            thunk_FUN_0050e3a0((LPVOID)(local_16a0 + -0xc));
            lVar17 = local_1778;
          }
        }
        piVar20 = (int *)(lVar17 + -0xc);
        if (piVar20 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar8 = *piVar20;
          *piVar20 = *piVar20 + -1;
          UNLOCK();
          if (iVar8 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar20);
          }
        }
        break;
      }
      piVar20 = (int *)(local_1778 + -0xc);
      if (piVar20 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar1 = *piVar20;
        *piVar20 = *piVar20 + -1;
        UNLOCK();
        if (iVar1 + -1 < 1) {
          thunk_FUN_0050e3a0(piVar20);
        }
      }
      lVar22 = lVar22 + 1;
      iVar8 = iVar8 + 1;
      ppcVar23 = ppcVar23 + 2;
    } while (lVar22 < 0xd);
    FUN_0040331c((longlong *)&local_1700,"-q");
    FUN_0040331c((longlong *)&local_1770,"http://q-dir.com/?");
    puVar18 = local_1770;
    local_16c8 = (undefined8 *)PTR_DAT_00546000;
    FUN_00402204((longlong *)&local_16c8,*(int *)(local_1770 + -1),local_1770,
                 *(int *)(local_1790 + -4),(undefined8 *)local_1790);
    puVar6 = local_16c8;
    local_16d0 = (LPCWSTR)PTR_DAT_00546000;
    FUN_00402204((longlong *)&local_16d0,*(int *)(local_16c8 + -1),local_16c8,
                 *(int *)(local_1700 + -1),local_1700);
    piVar20 = (int *)((longlong)puVar6 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      puVar18 = local_1770;
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
        puVar18 = local_1770;
      }
    }
    piVar20 = (int *)((longlong)puVar18 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
      }
    }
    piVar20 = (int *)((longlong)local_1700 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
      }
    }
    pWVar11 = local_16d0;
    param_4 = 0;
    param_3 = local_16d0;
    ShellExecuteW(*(HWND *)(param_1 + 8),L"open",local_16d0,(LPCWSTR)0x0,(LPCWSTR)0x0,3);
    FUN_00402010((longlong *)&local_1790);
    CharLowerW(local_1790);
    pWVar11 = pWVar11 + -6;
    if (pWVar11 != (LPCWSTR)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *(int *)pWVar11;
      *(int *)pWVar11 = *(int *)pWVar11 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(pWVar11);
      }
    }
    local_1790 = local_1790 + -6;
    if (local_1790 != (LPWSTR)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *(int *)local_1790;
      *(int *)local_1790 = *(int *)local_1790 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(local_1790);
      }
    }
    piVar20 = (int *)(local_1788 + -6);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      local_16b0 = local_1788;
      goto joined_r0x004648c8;
    }
  }
  puVar18 = local_1798;
  if (*(int *)(local_1798 + -1) < 4) {
    if (sVar19 == -0x7fd1) {
      FUN_00455e1c(*(undefined8 *)(param_1 + 8));
      goto LAB_00464dff;
    }
    if (sVar19 == -0x7fd0) {
      local_16e0 = 0;
      bVar7 = FUN_00456048(param_1,&local_16e0);
      if ((int)CONCAT71(extraout_var,bVar7) != 0) {
        FUN_0044e63c(&local_1538);
        local_1530 = local_16e0;
        plVar13 = FUN_0049feec(&local_15b0);
        FUN_00451318((longlong)&local_1538,plVar13);
        FUN_0044e34c(&local_1538);
      }
      goto LAB_00464dff;
    }
    if (sVar19 == -0x7fcf) {
      FUN_004e805c(&local_16e8);
      FUN_0040331c(&local_16f8,"");
      puVar9 = FUN_004a3f04(&local_1710,0xe,param_3,param_4);
      local_1738 = (undefined8 *)PTR_DAT_00546000;
      iVar8 = lstrlenW(L": ");
      FUN_00402204((longlong *)&local_1738,*(int *)((undefined8 *)*puVar9 + -1),
                   (undefined8 *)*puVar9,iVar8,(undefined8 *)&DAT_0052ad6c);
      plVar13 = FUN_0049feec(&local_1708);
      puVar18 = local_1738;
      local_1760 = PTR_DAT_00546000;
      FUN_00402204((longlong *)&local_1760,*(int *)(local_1738 + -1),local_1738,
                   *(int *)((undefined8 *)*plVar13 + -1),(undefined8 *)*plVar13);
      piVar20 = (int *)((longlong)puVar18 + -0xc);
      if (piVar20 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar8 = *piVar20;
        *piVar20 = *piVar20 + -1;
        UNLOCK();
        if (iVar8 + -1 < 1) {
          thunk_FUN_0050e3a0(piVar20);
        }
      }
      piVar20 = (int *)(local_1710 - 0xc);
      if (piVar20 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar8 = *piVar20;
        *piVar20 = *piVar20 + -1;
        UNLOCK();
        if (iVar8 + -1 < 1) {
          thunk_FUN_0050e3a0((LPVOID)(local_1710 - 0xc));
        }
      }
      piVar20 = (int *)(local_1708 + -0xc);
      if (piVar20 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar8 = *piVar20;
        *piVar20 = *piVar20 + -1;
        UNLOCK();
        if (iVar8 + -1 < 1) {
          thunk_FUN_0050e3a0((LPVOID)(local_1708 + -0xc));
        }
      }
      plVar13 = FUN_0040329c(&local_15a8,(longlong *)&local_1760);
      plVar14 = FUN_0040329c(&local_1590,&local_16f8);
      FUN_004e9444(&local_16e8,*(HWND *)(param_1 + 8),plVar14,plVar13);
      piVar20 = (int *)(local_1760 + -0xc);
      if (piVar20 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar8 = *piVar20;
        *piVar20 = *piVar20 + -1;
        UNLOCK();
        if (iVar8 + -1 < 1) {
          thunk_FUN_0050e3a0(piVar20);
        }
      }
      piVar20 = (int *)(local_16f8 + -0xc);
      if (piVar20 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar8 = *piVar20;
        *piVar20 = *piVar20 + -1;
        UNLOCK();
        if (iVar8 + -1 < 1) {
          thunk_FUN_0050e3a0(piVar20);
        }
      }
      FUN_004e806c(&local_16e8);
      goto LAB_00464dff;
    }
    if (sVar19 != -0x7fce) goto LAB_00464dff;
    FUN_0049f8f8(&local_1740,0);
    FUN_004e805c(&local_1720);
    puVar9 = FUN_004a3f04(&local_16c0,0xe,param_3,param_4);
    local_16d8 = (undefined8 *)PTR_DAT_00546000;
    iVar8 = lstrlenW(L": ");
    FUN_00402204((longlong *)&local_16d8,*(int *)((undefined8 *)*puVar9 + -1),(undefined8 *)*puVar9,
                 iVar8,(undefined8 *)&DAT_0052ad6c);
    plVar13 = FUN_0049feec(&local_1730);
    puVar18 = local_16d8;
    local_1750 = PTR_DAT_00546000;
    FUN_00402204((longlong *)&local_1750,*(int *)(local_16d8 + -1),local_16d8,
                 *(int *)((undefined8 *)*plVar13 + -1),(undefined8 *)*plVar13);
    piVar20 = (int *)((longlong)puVar18 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
      }
    }
    piVar20 = (int *)(local_16c0 - 0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_16c0 - 0xc));
      }
    }
    piVar20 = (int *)(local_1730 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_1730 + -0xc));
      }
    }
    plVar13 = FUN_0040329c(&local_15b8,(longlong *)&local_1750);
    plVar14 = FUN_0040329c(&local_15c0,&local_1740);
    FUN_004e9444(&local_1720,*(HWND *)(param_1 + 8),plVar14,plVar13);
    piVar20 = (int *)(local_1750 + -0xc);
    if (piVar20 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *piVar20;
      *piVar20 = *piVar20 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar20);
      }
    }
    FUN_004e806c(&local_1720);
    piVar20 = (int *)(local_1740 + -0xc);
    if (piVar20 == (int *)PTR_DAT_00546008) goto LAB_00464dff;
    LOCK();
    iVar8 = *piVar20;
    *piVar20 = *piVar20 + -1;
    UNLOCK();
  }
  else {
    plVar13 = FUN_0049f19c(&local_16a8);
    local_16f0 = (LPCWSTR)PTR_DAT_00546000;
    FUN_00402204((longlong *)&local_16f0,*(int *)((undefined8 *)*plVar13 + -1),
                 (undefined8 *)*plVar13,*(int *)(puVar18 + -1),puVar18);
    pWVar11 = local_16f0;
    ShellExecuteW(*(HWND *)(param_1 + 8),L"open",local_16f0,(LPCWSTR)0x0,(LPCWSTR)0x0,5);
    pWVar11 = pWVar11 + -6;
    if (pWVar11 != (LPCWSTR)PTR_DAT_00546008) {
      LOCK();
      iVar8 = *(int *)pWVar11;
      *(int *)pWVar11 = *(int *)pWVar11 + -1;
      UNLOCK();
      if (iVar8 + -1 < 1) {
        thunk_FUN_0050e3a0(pWVar11);
      }
    }
    piVar20 = (int *)(local_16a8 + -0xc);
    if (piVar20 == (int *)PTR_DAT_00546008) goto LAB_00464dff;
    LOCK();
    iVar8 = *piVar20;
    *piVar20 = *piVar20 + -1;
    UNLOCK();
    local_1740 = local_16a8;
  }
  if (iVar8 + -1 < 1) {
    thunk_FUN_0050e3a0((LPVOID)(local_1740 + -0xc));
  }
LAB_00464dff:
  piVar20 = (int *)((longlong)local_1798 + -0xc);
  if (piVar20 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar8 = *piVar20;
    *piVar20 = *piVar20 + -1;
    UNLOCK();
    if (iVar8 + -1 < 1) {
      thunk_FUN_0050e3a0(piVar20);
    }
  }
  return 0;
}


