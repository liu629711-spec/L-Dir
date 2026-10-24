// ==== FUN_00497c64 @ 00497c64 ====

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined8 FUN_00497c64(IDropTargetVtbl *param_1,int param_2,HWND param_3)

{
  LPCITEMIDLIST pIVar1;
  LPCWSTR pWVar2;
  longlong lVar3;
  LPCITEMIDLIST pidl;
  bool bVar4;
  BOOL BVar5;
  int iVar6;
  uint uVar7;
  HMENU pHVar8;
  ulonglong *puVar9;
  undefined8 *puVar10;
  undefined8 *puVar11;
  LPITEMIDLIST pIVar12;
  longlong *plVar13;
  HICON pHVar14;
  undefined7 extraout_var;
  undefined8 *puVar15;
  IDropTargetVtbl *pIVar16;
  undefined7 extraout_var_00;
  undefined7 extraout_var_01;
  LPVOID pvVar17;
  int iVar18;
  LPVOID pvVar19;
  undefined8 uVar20;
  undefined4 *lParam;
  int *piVar21;
  int iVar22;
  LPVOID pvVar23;
  ulonglong uVar24;
  LPVOID pvVar25;
  code *pcVar26;
  int local_1a8 [2];
  tagPOINT local_1a0;
  IDropTargetVtbl *local_198;
  LONG local_190;
  LONG LStack_18c;
  LONG local_188;
  LONG LStack_184;
  int local_180 [2];
  undefined8 *local_178;
  undefined8 *local_170;
  undefined8 *local_168;
  LPCITEMIDLIST local_160;
  code *local_158;
  longlong local_150;
  longlong local_148;
  LONG local_140;
  LONG LStack_13c;
  longlong local_138;
  longlong local_130;
  LPCWSTR local_128;
  ulonglong local_120;
  LPCWSTR local_118;
  undefined8 *local_110;
  longlong local_108;
  longlong local_100;
  undefined8 *local_f8;
  LPVOID local_f0;
  LPVOID local_e8;
  int local_e0;
  LPVOID local_d8;
  LPVOID local_d0;
  int local_c8;
  undefined8 local_c0 [2];
  undefined **local_b0;
  undefined8 local_a8;
  LONG local_a0 [3];
  LONG local_94;
  longlong local_90;
  undefined8 local_88 [2];
  undefined4 local_78;
  int local_74 [7];
  undefined8 local_58;
  
  local_58 = DAT_00547f48;
  local_198 = param_1;
  SendMessageW(param_3,0x41d,(longlong)param_2,(LPARAM)local_a0);
  local_1a0.x = local_a0[0];
  local_1a0.y = local_94;
  ClientToScreen(param_3,&local_1a0);
  lVar3 = *(longlong *)(param_1[7].AddRef + 0x90);
  local_78 = 0;
  FUN_0050daf0((ulonglong *)local_74,0,0x1c);
  lParam = &local_78;
  SendMessageW(param_3,0x417,(longlong)param_2,(LPARAM)lParam);
  pcVar26 = param_1[7].AddRef + 0x780;
  local_158 = pcVar26;
  if (local_74[0] == 0xa772) {
    FUN_004045dc(&local_a8,
                 (longlong *)
                 (-(ulonglong)(param_1 != (IDropTargetVtbl *)0xfffffffffffffd00) &
                 (ulonglong)&param_1[0xd].Drop));
    lParam = (undefined4 *)0x0;
    local_b0 = &PTR_FUN_00527d50;
    FUN_004cebf4((longlong)param_1[7].AddRef,&local_b0,-1,(LPCITEMIDLIST)0x0);
  }
  if (local_74[0] == 0xa770) {
    uVar20 = 1;
    FUN_0041a270((longlong)pcVar26,0xa770,1);
    pHVar8 = CreatePopupMenu();
    puVar9 = FUN_004a3f04(&local_120,0xcf10,uVar20,(ulonglong)lParam);
    AppendMenuW(pHVar8,3,0xffffffffffffffff,(LPCWSTR)*puVar9);
    piVar21 = (int *)(local_120 - 0xc);
    if (piVar21 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar18 = *piVar21;
      *piVar21 = *piVar21 + -1;
      UNLOCK();
      if (iVar18 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_120 - 0xc));
      }
    }
    AppendMenuW(pHVar8,0x800,0,(LPCWSTR)0x0);
    AppendMenuW(pHVar8,0,0x8041,L"# 1");
    AppendMenuW(pHVar8,0,0x8042,L"# 2");
    AppendMenuW(pHVar8,0,0x8043,L"# 3");
    AppendMenuW(pHVar8,0,0x8044,L"# 4");
    puVar15 = (undefined8 *)*DAT_005541f0;
    AppendMenuW(pHVar8,0x800,0,(LPCWSTR)0x0);
    iVar18 = 60000;
    while (puVar15 != DAT_005541f0) {
      AppendMenuW(pHVar8,0,60000,(LPCWSTR)puVar15[3]);
      puVar10 = (undefined8 *)puVar15[2];
      if (puVar10 == DAT_0054bd20) {
        puVar10 = (undefined8 *)puVar15[1];
        while (puVar11 = puVar10, puVar15 == (undefined8 *)puVar11[2]) {
          puVar15 = puVar11;
          puVar10 = (undefined8 *)puVar11[1];
        }
        if ((undefined8 *)puVar15[2] != puVar11) {
          puVar15 = puVar11;
        }
      }
      else {
        puVar11 = (undefined8 *)*puVar10;
        puVar15 = puVar10;
        while (puVar10 = puVar11, puVar10 != DAT_0054bd20) {
          puVar15 = puVar10;
          puVar11 = (undefined8 *)*puVar10;
        }
      }
    }
    local_140 = local_1a0.x;
    LStack_13c = local_1a0.y;
    BVar5 = FUN_00507844(lVar3,pHVar8,CONCAT44(local_1a0.y,local_1a0.x),0x102);
    if (BVar5 < 60000) {
      if (BVar5 != 0) {
        FUN_00483718((longlong)param_1,BVar5);
      }
    }
    else {
      puVar15 = (undefined8 *)*DAT_005541f0;
      while (puVar15 != DAT_005541f0) {
        if (iVar18 == BVar5) {
          plVar13 = FUN_0040329c(&local_90,puVar15 + 3);
          FUN_004937c4(param_1,plVar13,0);
          break;
        }
        iVar18 = iVar18 + 1;
        puVar10 = (undefined8 *)puVar15[2];
        if ((undefined8 *)puVar15[2] == DAT_0054bd20) {
          puVar10 = (undefined8 *)puVar15[1];
          while (puVar11 = puVar10, puVar15 == (undefined8 *)puVar11[2]) {
            puVar15 = puVar11;
            puVar10 = (undefined8 *)puVar11[1];
          }
          if ((undefined8 *)puVar15[2] != puVar11) {
            puVar15 = puVar11;
          }
        }
        else {
          do {
            puVar15 = puVar10;
            puVar10 = (undefined8 *)*puVar15;
          } while ((undefined8 *)*puVar15 != DAT_0054bd20);
        }
      }
    }
    FUN_0041a270((longlong)pcVar26,0xa770,0);
    if (pHVar8 == (HMENU)0x0) {
      return 1;
    }
    DestroyMenu(pHVar8);
  }
  if ((local_74[0] == 0xa76c) || (local_74[0] == 0xa76d)) {
    pHVar8 = CreatePopupMenu();
    if (local_74[0] == 0xa76c) {
      local_1a8[0] = *(int *)&param_1[10].DragEnter;
      while (local_1a8[0] = local_1a8[0] + -1, -1 < local_1a8[0]) {
        pIVar16 = (IDropTargetVtbl *)FUN_00496b4c((longlong)&param_1[0xe].DragOver,local_1a8);
        local_198 = pIVar16;
        FUN_0040331c((longlong *)&local_110,"\t");
        plVar13 = FUN_004ef164(&local_130,(LPCWSTR)pIVar16->AddRef);
        local_170 = (undefined8 *)PTR_DAT_00546000;
        FUN_00402204((longlong *)&local_170,*(int *)((undefined8 *)*plVar13 + -1),
                     (undefined8 *)*plVar13,*(int *)(local_110 + -1),local_110);
        plVar13 = FUN_004ef70c((longlong)pIVar16,&local_150);
        puVar15 = local_170;
        local_128 = (LPCWSTR)PTR_DAT_00546000;
        FUN_00402204((longlong *)&local_128,*(int *)(local_170 + -1),local_170,
                     *(int *)((undefined8 *)*plVar13 + -1),(undefined8 *)*plVar13);
        pWVar2 = local_128;
        iVar18 = local_1a8[0] + 1;
        AppendMenuW(pHVar8,0,(longlong)iVar18,local_128);
        pWVar2 = pWVar2 + -6;
        if (pWVar2 != (LPCWSTR)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *(int *)pWVar2;
          *(int *)pWVar2 = *(int *)pWVar2 + -1;
          UNLOCK();
          puVar15 = local_170;
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0(pWVar2);
            puVar15 = local_170;
          }
        }
        piVar21 = (int *)((longlong)puVar15 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar21);
          }
        }
        piVar21 = (int *)(local_130 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0((LPVOID)(local_130 + -0xc));
          }
        }
        piVar21 = (int *)(local_150 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0((LPVOID)(local_150 + -0xc));
          }
        }
        piVar21 = (int *)((longlong)local_110 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar21);
          }
        }
        iVar6 = FUN_004ef058((longlong)local_198,1);
        pHVar14 = FUN_005047fc(iVar6,0);
        bVar4 = FUN_00508340(lVar3,pHVar14,iVar18);
        if ((int)CONCAT71(extraout_var_00,bVar4) == 0) {
          FUN_00508390(lVar3,pHVar14,iVar18);
        }
      }
    }
    else if ((local_74[0] == 0xa76d) &&
            (iVar18 = *(int *)&param_1[10].DragEnter + 1, local_180[0] = iVar18,
            iVar18 < *(int *)&param_1[0xf].AddRef + 1)) {
      do {
        local_180[0] = iVar18;
        pIVar16 = (IDropTargetVtbl *)FUN_00496b4c((longlong)&param_1[0xe].DragOver,local_180);
        local_198 = pIVar16;
        FUN_0040331c((longlong *)&local_f8,"\t");
        plVar13 = FUN_004ef164(&local_100,(LPCWSTR)pIVar16->AddRef);
        local_178 = (undefined8 *)PTR_DAT_00546000;
        FUN_00402204((longlong *)&local_178,*(int *)((undefined8 *)*plVar13 + -1),
                     (undefined8 *)*plVar13,*(int *)(local_f8 + -1),local_f8);
        plVar13 = FUN_004ef70c((longlong)pIVar16,&local_108);
        puVar15 = local_178;
        local_118 = (LPCWSTR)PTR_DAT_00546000;
        FUN_00402204((longlong *)&local_118,*(int *)(local_178 + -1),local_178,
                     *(int *)((undefined8 *)*plVar13 + -1),(undefined8 *)*plVar13);
        pWVar2 = local_118;
        iVar18 = iVar18 + 1;
        AppendMenuW(pHVar8,0,(longlong)iVar18,local_118);
        pWVar2 = pWVar2 + -6;
        if (pWVar2 != (LPCWSTR)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *(int *)pWVar2;
          *(int *)pWVar2 = *(int *)pWVar2 + -1;
          UNLOCK();
          puVar15 = local_178;
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0(pWVar2);
            puVar15 = local_178;
          }
        }
        piVar21 = (int *)((longlong)puVar15 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar21);
          }
        }
        piVar21 = (int *)(local_100 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0((LPVOID)(local_100 + -0xc));
          }
        }
        piVar21 = (int *)(local_108 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0((LPVOID)(local_108 + -0xc));
          }
        }
        piVar21 = (int *)((longlong)local_f8 + -0xc);
        if (piVar21 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar6 = *piVar21;
          *piVar21 = *piVar21 + -1;
          UNLOCK();
          if (iVar6 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar21);
          }
        }
        iVar6 = FUN_004ef058((longlong)local_198,1);
        pHVar14 = FUN_005047fc(iVar6,0);
        bVar4 = FUN_00508340(lVar3,pHVar14,iVar18);
        if ((int)CONCAT71(extraout_var_01,bVar4) == 0) {
          FUN_00508390(lVar3,pHVar14,iVar18);
        }
        local_180[0] = iVar18;
      } while (iVar18 < *(int *)&param_1[0xf].AddRef + 1);
    }
    FUN_0041a270((longlong)pcVar26,local_74[0],1);
    iVar18 = GetMenuItemCount(pHVar8);
    if (0 < iVar18) {
      uVar24 = 0;
      AppendMenuW(pHVar8,0x800,0,(LPCWSTR)0x0);
      pHVar14 = FUN_005048a4(10,0);
      uVar20 = 0xffff;
      FUN_00508390(lVar3,pHVar14,0xffff);
      puVar9 = FUN_004a3f04((ulonglong *)&local_188,0x13,uVar20,uVar24);
      AppendMenuW(pHVar8,0,0xffff,(LPCWSTR)*puVar9);
      piVar21 = (int *)(CONCAT44(LStack_184,local_188) + -0xc);
      if (piVar21 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar18 = *piVar21;
        *piVar21 = *piVar21 + -1;
        UNLOCK();
        if (iVar18 + -1 < 1) {
          thunk_FUN_0050e3a0((LPVOID)(CONCAT44(LStack_184,local_188) + -0xc));
        }
      }
    }
    local_188 = local_1a0.x;
    LStack_184 = local_1a0.y;
    BVar5 = FUN_00507844(lVar3,pHVar8,CONCAT44(local_1a0.y,local_1a0.x),0x102);
    if (BVar5 == 0xffff) {
      FUN_00492f00((longlong)param_1);
    }
    else if (BVar5 != 0) {
      FUN_00497254(param_1,BVar5 + -1);
    }
    FUN_0041a288((longlong)pcVar26,local_74[0],0);
    if (pHVar8 != (HMENU)0x0) {
      DestroyMenu(pHVar8);
    }
  }
  else if (local_74[0] == 0xa771) {
    FUN_0041a288((longlong)pcVar26,0xa771,1);
    local_f0 = (LPVOID)0x0;
    local_e8 = (LPVOID)0x0;
    local_c8 = 0;
    local_e0 = 0;
    local_d8 = (LPVOID)0x0;
    local_d0 = (LPVOID)0x0;
    pHVar8 = CreatePopupMenu();
    iVar6 = FUN_004ef0d0((ushort *)param_1[0xd].Drop);
    pvVar17 = (LPVOID)0x0;
    pvVar19 = (LPVOID)0x0;
    pvVar23 = (LPVOID)0x0;
    pvVar25 = (LPVOID)0x0;
    iVar18 = 0;
    if (iVar6 != 0) {
      local_160 = ILClone((LPCITEMIDLIST)param_1[0xd].Drop);
      uVar7 = FUN_004ef0d0((ushort *)param_1[0xd].Drop);
      pidl = local_160;
      iVar18 = 0;
      if (0 < (int)uVar7) {
        uVar24 = (ulonglong)uVar7;
        iVar6 = 0;
        do {
          iVar22 = iVar6 + 1;
          pIVar12 = ILFindLastID(pidl);
          pIVar12 = ILClone(pIVar12);
          FUN_00486cc8((longlong *)&local_d8,iVar6,pIVar12);
          ILRemoveLastID(pidl);
          pIVar12 = ILClone(pidl);
          FUN_00486cc8((longlong *)&local_f0,iVar6,pIVar12);
          FUN_004efe84(local_c0,(ushort *)pidl);
          FUN_0040331c((longlong *)&local_190,"\t");
          plVar13 = FUN_004ef164(&local_148,(LPCWSTR)pidl);
          local_168 = (undefined8 *)PTR_DAT_00546000;
          FUN_00402204((longlong *)&local_168,*(int *)((undefined8 *)*plVar13 + -1),
                       (undefined8 *)*plVar13,
                       *(int *)((undefined8 *)CONCAT44(LStack_18c,local_190) + -1),
                       (undefined8 *)CONCAT44(LStack_18c,local_190));
          plVar13 = FUN_004ef70c((longlong)local_c0,&local_138);
          puVar15 = local_168;
          local_160 = (LPCITEMIDLIST)PTR_DAT_00546000;
          FUN_00402204((longlong *)&local_160,*(int *)(local_168 + -1),local_168,
                       *(int *)((undefined8 *)*plVar13 + -1),(undefined8 *)*plVar13);
          pIVar1 = local_160;
          AppendMenuW(pHVar8,0,(longlong)iVar22,(LPCWSTR)local_160);
          pIVar1 = pIVar1 + -4;
          if (pIVar1 != (LPCITEMIDLIST)PTR_DAT_00546008) {
            LOCK();
            iVar18 = *(int *)pIVar1;
            *(int *)pIVar1 = *(int *)pIVar1 + -1;
            UNLOCK();
            puVar15 = local_168;
            if (iVar18 + -1 < 1) {
              thunk_FUN_0050e3a0(pIVar1);
              puVar15 = local_168;
            }
          }
          piVar21 = (int *)((longlong)puVar15 + -0xc);
          if (piVar21 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar18 = *piVar21;
            *piVar21 = *piVar21 + -1;
            UNLOCK();
            if (iVar18 + -1 < 1) {
              thunk_FUN_0050e3a0(piVar21);
            }
          }
          piVar21 = (int *)(local_148 + -0xc);
          if (piVar21 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar18 = *piVar21;
            *piVar21 = *piVar21 + -1;
            UNLOCK();
            if (iVar18 + -1 < 1) {
              thunk_FUN_0050e3a0((LPVOID)(local_148 + -0xc));
            }
          }
          piVar21 = (int *)(local_138 + -0xc);
          if (piVar21 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar18 = *piVar21;
            *piVar21 = *piVar21 + -1;
            UNLOCK();
            if (iVar18 + -1 < 1) {
              thunk_FUN_0050e3a0((LPVOID)(local_138 + -0xc));
            }
          }
          piVar21 = (int *)(CONCAT44(LStack_18c,local_190) + -0xc);
          if (piVar21 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar18 = *piVar21;
            *piVar21 = *piVar21 + -1;
            UNLOCK();
            if (iVar18 + -1 < 1) {
              thunk_FUN_0050e3a0(piVar21);
            }
          }
          iVar18 = FUN_004ef058((longlong)local_c0,1);
          pHVar14 = FUN_005047fc(iVar18,0);
          bVar4 = FUN_00508340(lVar3,pHVar14,iVar22);
          if ((int)CONCAT71(extraout_var,bVar4) == 0) {
            FUN_00508390(lVar3,pHVar14,iVar22);
          }
          FUN_004efeb4(local_c0);
          uVar24 = uVar24 - 1;
          pvVar17 = local_e8;
          param_1 = local_198;
          pvVar19 = local_d0;
          pvVar23 = local_f0;
          pvVar25 = local_d8;
          iVar18 = local_e0;
          iVar6 = iVar22;
        } while (uVar24 != 0);
      }
    }
    local_190 = local_1a0.x;
    LStack_18c = local_1a0.y;
    BVar5 = FUN_00507844(lVar3,pHVar8,CONCAT44(local_1a0.y,local_1a0.x),0x102);
    FUN_0041a288((longlong)local_158,0xa771,0);
    if (BVar5 != 0) {
      iVar6 = BVar5 + -1;
      if ((iVar6 < 0) || (iVar18 <= iVar6)) {
        RaiseException(0xc000008c,1,0,(ULONG_PTR *)0x0);
        pcVar26 = (code *)swi(3);
        uVar20 = (*pcVar26)();
        return uVar20;
      }
      if ((iVar6 < 0) || (local_c8 <= iVar6)) {
        RaiseException(0xc000008c,1,0,(ULONG_PTR *)0x0);
        pcVar26 = (code *)swi(3);
        uVar20 = (*pcVar26)();
        return uVar20;
      }
      uVar20 = *(undefined8 *)((longlong)pvVar19 + (longlong)iVar6 * 8);
      puVar15 = FUN_004efe84(local_88,*(ushort **)((longlong)pvVar17 + (longlong)iVar6 * 8));
      iVar18 = FUN_0048dffc(param_1,puVar15,0,1);
      if (iVar18 != 0) {
        FUN_00480b24((longlong)param_1,uVar20);
      }
    }
    if (pHVar8 != (HMENU)0x0) {
      DestroyMenu(pHVar8);
    }
    if (pvVar25 != (LPVOID)0x0) {
      FUN_0050e3a0(pvVar25);
    }
    if (pvVar19 != (LPVOID)0x0) {
      FUN_0050e3a0(pvVar19);
    }
    if (pvVar23 != (LPVOID)0x0) {
      FUN_0050e3a0(pvVar23);
    }
    if (pvVar17 != (LPVOID)0x0) {
      FUN_0050e3a0(pvVar17);
    }
  }
  return 1;
}


