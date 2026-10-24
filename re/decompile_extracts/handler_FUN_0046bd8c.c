// ==== FUN_0046bd8c @ 0046bd8c ====

undefined8
FUN_0046bd8c(longlong param_1,undefined8 param_2,uint param_3,undefined8 param_4,undefined4 *param_5
            )

{
  bool bVar1;
  bool bVar2;
  bool bVar3;
  bool bVar4;
  bool bVar5;
  BOOL BVar6;
  undefined8 *puVar7;
  int iVar8;
  uint uVar9;
  uint uVar10;
  int iVar11;
  
  uVar9 = param_3 & 0xffff;
  uVar10 = uVar9 - 0x8497;
  if (uVar10 < 5) {
    FUN_0043e768((undefined8 *)(param_1 + 0x7ad8),0);
    FUN_0043e768((undefined8 *)(param_1 + 0x7c28),1);
    FUN_0043e768((undefined8 *)(param_1 + 0x7b80),1);
  }
  bVar2 = false;
  if (uVar10 == 0) {
    FUN_00455b3c(param_1,1,1,1,1);
LAB_0046be09:
    iVar11 = 5;
    if (*(int *)(param_1 + 0x7b2c) != -1) {
      ShowWindow(*(HWND *)(param_1 + 0x7ad8 + (ulonglong)(*(int *)(param_1 + 0x7b2c) == 0) * 8),5);
    }
    *(undefined4 *)(param_1 + 0x7b2c) = 0xffffffff;
    FUN_0042a228((undefined8 *)(param_1 + 0x7ad8));
LAB_0046be4b:
    if (*(int *)(param_1 + 0x7bd4) != -1) {
      ShowWindow(*(HWND *)(param_1 + 0x7b80 + (ulonglong)(*(int *)(param_1 + 0x7bd4) == 0) * 8),
                 iVar11);
    }
    *(undefined4 *)(param_1 + 0x7bd4) = 0xffffffff;
LAB_0046be7b:
    FUN_0042a228((undefined8 *)(param_1 + 0x7b80));
    if (*(int *)(param_1 + 0x7c7c) != -1) {
      ShowWindow(*(HWND *)(param_1 + 0x7c28 + (ulonglong)(*(int *)(param_1 + 0x7c7c) == 0) * 8),
                 iVar11);
    }
    *(undefined4 *)(param_1 + 0x7c7c) = 0xffffffff;
  }
  else {
    bVar1 = false;
    if (uVar10 == 1) {
      FUN_00455b3c(param_1,1,0,1,1);
LAB_0046c1a0:
      iVar11 = 5;
      if (*(int *)(param_1 + 0x7b2c) != -1) {
        ShowWindow(*(HWND *)(param_1 + 0x7ad8 + (ulonglong)(*(int *)(param_1 + 0x7b2c) == 0) * 8),5)
        ;
      }
      *(undefined4 *)(param_1 + 0x7b2c) = 0xffffffff;
      FUN_0042a228((undefined8 *)(param_1 + 0x7ad8));
      BVar6 = IsWindowVisible(*(HWND *)(param_1 + 0x7b80));
      if (BVar6 == 0) {
        ShowWindow(*(HWND *)(param_1 + 0x7b80),5);
      }
      ShowWindow(*(HWND *)(param_1 + 0x7b88),0);
      if (*(int *)(param_1 + 0x7ba8) != 0) {
        *(undefined4 *)(param_1 + 0x7ba8) = 0;
      }
      *(undefined4 *)(param_1 + 0x7bd4) = 0;
      goto LAB_0046be7b;
    }
    if (uVar10 == 2) {
      FUN_00455b3c(param_1,1,1,1,0);
      iVar11 = uVar9 - 0x8494;
LAB_0046c280:
      if (*(int *)(param_1 + 0x7b2c) != -1) {
        ShowWindow(*(HWND *)(param_1 + 0x7ad8 + (ulonglong)(*(int *)(param_1 + 0x7b2c) == 0) * 8),
                   iVar11);
      }
      *(undefined4 *)(param_1 + 0x7b2c) = 0xffffffff;
      FUN_0042a228((undefined8 *)(param_1 + 0x7ad8));
      if (*(int *)(param_1 + 0x7bd4) != -1) {
        ShowWindow(*(HWND *)(param_1 + 0x7b80 + (ulonglong)(*(int *)(param_1 + 0x7bd4) == 0) * 8),
                   iVar11);
      }
      *(undefined4 *)(param_1 + 0x7bd4) = 0xffffffff;
    }
    else {
      if (uVar10 == 3) {
        FUN_00455b3c(param_1,1,1,0,0);
        BVar6 = IsWindowVisible(*(HWND *)(param_1 + 0x7ad8));
        iVar11 = uVar9 - 0x8495;
        if (BVar6 == 0) {
          ShowWindow(*(HWND *)(param_1 + 0x7ad8),iVar11);
        }
        ShowWindow(*(HWND *)(param_1 + 0x7ae0),0);
        if (*(int *)(param_1 + 0x7b00) != 0) {
          *(undefined4 *)(param_1 + 0x7b00) = 0;
        }
        *(undefined4 *)(param_1 + 0x7b2c) = 0;
        FUN_0042a228((undefined8 *)(param_1 + 0x7ad8));
        goto LAB_0046be4b;
      }
      if (uVar10 != 4) {
        iVar11 = 5;
        if (uVar10 == 5) {
          FUN_00455b3c(param_1,1,1,1,1);
          FUN_0043e768((undefined8 *)(param_1 + 0x7ad8),1);
          FUN_0043e768((undefined8 *)(param_1 + 0x7c28),1);
          iVar11 = 1;
        }
        else {
          if (uVar10 != 6) {
            if (uVar10 == 7) {
              FUN_00455b3c(param_1,1,1,1,0);
              FUN_0043e768((undefined8 *)(param_1 + 0x7ad8),1);
              FUN_0043e768((undefined8 *)(param_1 + 0x7c28),1);
              puVar7 = (undefined8 *)(param_1 + 0x7b80);
LAB_0046c278:
              iVar8 = 1;
            }
            else {
              if (uVar10 != 8) {
                if (uVar10 == 9) {
                  FUN_00455b3c(param_1,1,0,1,1);
                  FUN_0043e768((undefined8 *)(param_1 + 0x7ad8),1);
                  FUN_0043e768((undefined8 *)(param_1 + 0x7b80),1);
                  FUN_0043e768((undefined8 *)(param_1 + 0x7c28),0);
                  goto LAB_0046c1a0;
                }
                if (uVar10 == 10) {
                  FUN_00455b3c(param_1,1,1,1,0);
                  FUN_0043e768((undefined8 *)(param_1 + 0x7ad8),1);
                  FUN_0043e768((undefined8 *)(param_1 + 0x7b80),0);
                  puVar7 = (undefined8 *)(param_1 + 0x7c28);
                  goto LAB_0046c278;
                }
                if (uVar10 == 0xb) {
                  FUN_00455b3c(param_1,1,0,0,0);
                  BVar6 = IsWindowVisible(*(HWND *)(param_1 + 0x7ad8));
                  if (BVar6 == 0) {
                    ShowWindow(*(HWND *)(param_1 + 0x7ad8),5);
                  }
                  ShowWindow(*(HWND *)(param_1 + 0x7ae0),0);
                  if (*(int *)(param_1 + 0x7b00) != 0) {
                    *(undefined4 *)(param_1 + 0x7b00) = 0;
                  }
                  *(undefined4 *)(param_1 + 0x7b2c) = 0;
                  FUN_0042a228((undefined8 *)(param_1 + 0x7ad8));
                  BVar6 = IsWindowVisible(*(HWND *)(param_1 + 0x7b80));
                  if (BVar6 == 0) {
                    ShowWindow(*(HWND *)(param_1 + 0x7b80),5);
                  }
                  ShowWindow(*(HWND *)(param_1 + 0x7b88),0);
                  if (*(int *)(param_1 + 0x7ba8) != 0) {
                    *(undefined4 *)(param_1 + 0x7ba8) = 0;
                  }
                  *(undefined4 *)(param_1 + 0x7bd4) = 0;
                  FUN_0042a228((undefined8 *)(param_1 + 0x7b80));
                  BVar6 = IsWindowVisible(*(HWND *)(param_1 + 0x7c28));
                  if (BVar6 == 0) {
                    ShowWindow(*(HWND *)(param_1 + 0x7c28),5);
                  }
                  ShowWindow(*(HWND *)(param_1 + 0x7c30),0);
                  if (*(int *)(param_1 + 0x7c50) != 0) {
                    *(undefined4 *)(param_1 + 0x7c50) = 0;
                  }
                  *(undefined4 *)(param_1 + 0x7c7c) = 0;
                  FUN_0042a228((undefined8 *)(param_1 + 0x7c28));
                }
                if (((int)uVar10 < 0x8497) || (0x84a2 < (int)uVar10)) goto LAB_0046c57f;
                bVar4 = false;
                if ((uVar10 == 0x8497) || ((uVar10 == 0x849c || (uVar10 == 0x849d)))) {
                  bVar3 = true;
LAB_0046c50e:
                  bVar1 = true;
                  bVar2 = true;
LAB_0046c514:
                  if ((uVar10 == 0x849e) || (uVar10 == 0x849f)) {
                    *(undefined4 *)(param_1 + 0x7b20) = 0x19c8;
                    FUN_0042a4dc((undefined8 *)(param_1 + 0x7ad8),(undefined8 *)0x0,'\x01');
                  }
                  else {
                    bVar5 = bVar2;
                    if (bVar1) {
LAB_0046c520:
                      bVar2 = bVar5;
                      FUN_0042a3bc((undefined8 *)(param_1 + 0x7ad8),-1,'\x01');
                    }
                  }
                  if (bVar2) goto LAB_0046c557;
                }
                else {
                  if (uVar10 == 0x8498) {
LAB_0046c506:
                    bVar3 = true;
                    bVar5 = false;
                    goto LAB_0046c520;
                  }
                  bVar3 = bVar2;
                  if (((uVar10 == 0x8499) || (bVar3 = bVar4, uVar10 == 0x849e)) ||
                     (uVar10 == 0x849f)) goto LAB_0046c50e;
                  if (uVar10 != 0x849a) {
                    bVar3 = bVar2;
                    bVar5 = false;
                    if (uVar10 != 0x849b) {
                      if (uVar10 != 0x84a0) {
                        if (uVar10 == 0x84a1) goto LAB_0046c506;
                        goto LAB_0046c514;
                      }
                      bVar3 = bVar4;
                      bVar5 = true;
                    }
                    goto LAB_0046c520;
                  }
                  bVar3 = true;
LAB_0046c557:
                  FUN_0042a3bc((undefined8 *)(param_1 + 0x7b80),-1,'\x01');
                }
                if (bVar3) {
                  FUN_0042a3bc((undefined8 *)(param_1 + 0x7c28),-1,'\x01');
                }
                goto LAB_0046c57f;
              }
              FUN_00455b3c(param_1,1,1,1,0);
              FUN_0043e768((undefined8 *)(param_1 + 0x7ad8),0);
              FUN_0043e768((undefined8 *)(param_1 + 0x7c28),0);
              iVar8 = 0;
              puVar7 = (undefined8 *)(param_1 + 0x7b80);
            }
            FUN_0043e768(puVar7,iVar8);
            goto LAB_0046c280;
          }
          FUN_00455b3c(param_1,1,1,1,1);
          FUN_0043e768((undefined8 *)(param_1 + 0x7ad8),0);
          FUN_0043e768((undefined8 *)(param_1 + 0x7c28),0);
          iVar11 = 0;
        }
        FUN_0043e768((undefined8 *)(param_1 + 0x7b80),iVar11);
        goto LAB_0046be09;
      }
      FUN_00455b3c(param_1,1,0,1,0);
      iVar11 = uVar9 - 0x8496;
      if (*(int *)(param_1 + 0x7b2c) != uVar9 - 0x849c) {
        ShowWindow(*(HWND *)(param_1 + 0x7ad8 + (ulonglong)(*(int *)(param_1 + 0x7b2c) == 0) * 8),
                   iVar11);
      }
      *(uint *)(param_1 + 0x7b2c) = uVar9 - 0x849c;
      FUN_0042a228((undefined8 *)(param_1 + 0x7ad8));
      BVar6 = IsWindowVisible(*(HWND *)(param_1 + 0x7b80));
      if (BVar6 == 0) {
        ShowWindow(*(HWND *)(param_1 + 0x7b80),iVar11);
      }
      ShowWindow(*(HWND *)(param_1 + 0x7b88),0);
      if (*(int *)(param_1 + 0x7ba8) != 0) {
        *(undefined4 *)(param_1 + 0x7ba8) = 0;
      }
      *(undefined4 *)(param_1 + 0x7bd4) = 0;
    }
    FUN_0042a228((undefined8 *)(param_1 + 0x7b80));
    BVar6 = IsWindowVisible(*(HWND *)(param_1 + 0x7c28));
    if (BVar6 == 0) {
      ShowWindow(*(HWND *)(param_1 + 0x7c28),iVar11);
    }
    ShowWindow(*(HWND *)(param_1 + 0x7c30),0);
    if (*(int *)(param_1 + 0x7c50) != 0) {
      *(undefined4 *)(param_1 + 0x7c50) = 0;
    }
    *(undefined4 *)(param_1 + 0x7c7c) = 0;
  }
  FUN_0042a228((undefined8 *)(param_1 + 0x7c28));
LAB_0046c57f:
  *(uint *)(param_1 + 0xc0c) = uVar10;
  *param_5 = 1;
  return 0;
}


// ==== thunk_FUN_00469284 @ 0046c5a8 ====

void thunk_FUN_00469284(longlong *param_1,HMENU param_2)

{
  int iVar1;
  longlong **pplVar2;
  undefined8 *puVar3;
  undefined *puVar4;
  char cVar5;
  int iVar6;
  undefined8 uVar7;
  HMENU hMenu;
  ulonglong *puVar8;
  int *piVar9;
  LPCWSTR pWVar10;
  uint *puVar11;
  undefined8 *puVar12;
  int iVar13;
  int iVar14;
  longlong *plVar15;
  undefined *uIDNewItem;
  longlong **lpNewItem;
  uint uVar16;
  int iStackX_20;
  longlong **pplStack_378;
  longlong **pplStack_370;
  longlong lStack_368;
  undefined8 *puStack_360;
  IMalloc *pIStack_358;
  undefined8 *puStack_350;
  undefined8 *puStack_348;
  longlong *plStack_340;
  undefined8 *puStack_338;
  undefined *puStack_330;
  longlong *plStack_328;
  undefined8 *puStack_320;
  undefined *puStack_318;
  undefined *puStack_310;
  undefined8 *puStack_308;
  undefined8 *puStack_300;
  undefined8 *puStack_2f8;
  undefined8 *puStack_2f0;
  undefined8 uStack_2e8;
  ulonglong uStack_2e0;
  short *psStack_2d8;
  undefined8 *puStack_2d0;
  undefined8 *puStack_2c8;
  LPCWSTR pWStack_2c0;
  undefined8 *puStack_2b8;
  longlong *plStack_2b0;
  undefined8 *puStack_2a8;
  undefined8 auStack_2a0 [3];
  longlong *plStack_288;
  int iStack_280;
  uint uStack_278;
  undefined4 uStack_274;
  longlong *plStack_168;
  int iStack_160;
  uint uStack_158;
  undefined4 uStack_154;
  
  psStack_2d8 = (short *)0x0;
  uStack_2e8 = 0;
  FUN_004becac(auStack_2a0);
  cVar5 = FUN_004be9b0(auStack_2a0,(LPCITEMIDLIST)*param_1,(longlong *)&psStack_2d8,&uStack_2e8);
  if (cVar5 != '\0') {
    FUN_0041c898(&plStack_340,psStack_2d8);
    FUN_0040331c(&lStack_368,"");
    iVar13 = 0;
    iStackX_20 = 0;
    plStack_2b0 = (longlong *)0x0;
    pIStack_358 = (IMalloc *)0x0;
    SHGetMalloc(&pIStack_358);
    iVar6 = (**(code **)*plStack_340)(plStack_340,&DAT_00534018,&plStack_2b0);
    if (iVar6 < 0) {
      plStack_328 = (longlong *)0x0;
      lpNewItem = &plStack_328;
      uIDNewItem = &DAT_005340c8;
      iVar6 = (**(code **)(*plStack_340 + 0x40))();
      if (-1 < iVar6) {
        iVar13 = 0;
        puVar4 = (undefined *)0x0;
        do {
          uIDNewItem = puVar4;
          FUN_0050daf0((ulonglong *)&plStack_168,0,0x118);
          lpNewItem = &plStack_168;
          iVar14 = (int)uIDNewItem;
          iVar6 = (**(code **)(*plStack_328 + 0x18))();
          if (-1 < iVar6) {
            pplStack_378 = (longlong **)PTR_DAT_00546000;
            if (iStack_160 == 0) {
              pWVar10 = (LPCWSTR)CONCAT44(uStack_154,uStack_158);
              if (pWVar10 == (LPCWSTR)0x0) {
                iVar6 = 0;
              }
              else {
                iVar6 = lstrlenW(pWVar10);
              }
              uVar7 = FUN_004020b8((longlong *)&pplStack_378,iVar6);
              pplVar2 = pplStack_378;
              if ((int)uVar7 != 0) {
                uIDNewItem = (undefined *)((longlong)iVar6 * 2);
                FUN_0050dc70(pplStack_378,(undefined8 *)pWVar10,(ulonglong)uIDNewItem);
                *(int *)(pplVar2 + -1) = iVar6;
                *(undefined2 *)((longlong)pplVar2 + (longlong)iVar6 * 2) = 0;
              }
              (*pIStack_358->lpVtbl->Free)(pIStack_358,(void *)CONCAT44(uStack_154,uStack_158));
            }
            else {
              if (iStack_160 == 1) {
                puVar11 = (uint *)((ulonglong)uStack_158 + *param_1);
              }
              else {
                if (iStack_160 != 2) goto LAB_004697d8;
                puVar11 = &uStack_158;
              }
              FUN_004033e8((longlong *)&pplStack_378,(LPCSTR)puVar11);
            }
LAB_004697d8:
            pplVar2 = pplStack_378;
            if (2 < *(int *)(pplStack_378 + -1)) {
              FUN_00402154(&DAT_005515f0 + iVar13,(longlong *)&pplStack_378);
              if (iVar13 == 0) {
                FUN_0040331c((longlong *)&puStack_320," ");
                puVar12 = puStack_320;
                puStack_2c8 = (undefined8 *)PTR_DAT_00546000;
                FUN_00402204((longlong *)&puStack_2c8,*(int *)(pplVar2 + -1),pplVar2,
                             *(int *)(puStack_320 + -1),puStack_320);
                puVar3 = puStack_2c8;
                FUN_004022a4(&lStack_368,*(int *)(puStack_2c8 + -1),puStack_2c8);
                piVar9 = (int *)((longlong)puVar3 + -0xc);
                if (piVar9 != (int *)PTR_DAT_00546008) {
                  LOCK();
                  iVar6 = *piVar9;
                  *piVar9 = *piVar9 + -1;
                  UNLOCK();
                  puVar12 = puStack_320;
                  if (iVar6 + -1 < 1) {
                    thunk_FUN_0050e3a0(piVar9);
                    puVar12 = puStack_320;
                  }
                }
                piVar9 = (int *)((longlong)puVar12 + -0xc);
                if (piVar9 != (int *)PTR_DAT_00546008) {
                  LOCK();
                  iVar6 = *piVar9;
                  *piVar9 = *piVar9 + -1;
                  UNLOCK();
                  if (iVar6 + -1 < 1) {
                    thunk_FUN_0050e3a0(piVar9);
                    goto LAB_004698b5;
                  }
                }
              }
              else {
LAB_004698b5:
                if (iVar13 == 1) {
                  FUN_0040329c((longlong *)&puStack_2f0,&lStack_368);
                  FUN_0040331c((longlong *)&puStack_2f8," ");
                  puStack_348 = (undefined8 *)PTR_DAT_00546000;
                  FUN_00402204((longlong *)&puStack_348,*(int *)(pplVar2 + -1),pplStack_378,
                               *(int *)(puStack_2f8 + -1),puStack_2f8);
                  puVar12 = puStack_348;
                  puStack_330 = PTR_DAT_00546000;
                  FUN_00402204((longlong *)&puStack_330,*(int *)(puStack_348 + -1),puStack_348,
                               *(int *)(puStack_2f0 + -1),puStack_2f0);
                  FUN_00402154(&lStack_368,(longlong *)&puStack_330);
                  piVar9 = (int *)(puStack_330 + -0xc);
                  if (piVar9 != (int *)PTR_DAT_00546008) {
                    LOCK();
                    iVar6 = *piVar9;
                    *piVar9 = *piVar9 + -1;
                    UNLOCK();
                    puVar12 = puStack_348;
                    if (iVar6 + -1 < 1) {
                      thunk_FUN_0050e3a0(piVar9);
                      puVar12 = puStack_348;
                    }
                  }
                  piVar9 = (int *)((longlong)puVar12 + -0xc);
                  if (piVar9 != (int *)PTR_DAT_00546008) {
                    LOCK();
                    iVar6 = *piVar9;
                    *piVar9 = *piVar9 + -1;
                    UNLOCK();
                    if (iVar6 + -1 < 1) {
                      thunk_FUN_0050e3a0(piVar9);
                    }
                  }
                  piVar9 = (int *)((longlong)puStack_2f8 + -0xc);
                  if (piVar9 != (int *)PTR_DAT_00546008) {
                    LOCK();
                    iVar6 = *piVar9;
                    *piVar9 = *piVar9 + -1;
                    UNLOCK();
                    if (iVar6 + -1 < 1) {
                      thunk_FUN_0050e3a0(piVar9);
                    }
                  }
                  piVar9 = (int *)((longlong)puStack_2f0 + -0xc);
                  if (piVar9 != (int *)PTR_DAT_00546008) {
                    LOCK();
                    iVar6 = *piVar9;
                    *piVar9 = *piVar9 + -1;
                    UNLOCK();
                    if (iVar6 + -1 < 1) {
                      thunk_FUN_0050e3a0(piVar9);
                    }
                  }
                }
              }
              uIDNewItem = (undefined *)(longlong)(DAT_00546c74 + iVar13);
              lpNewItem = pplStack_378;
              AppendMenuW(param_2,0,(UINT_PTR)uIDNewItem,(LPCWSTR)pplStack_378);
              iVar13 = iVar13 + 1;
            }
            piVar9 = (int *)((longlong)pplStack_378 + -0xc);
            if (piVar9 != (int *)PTR_DAT_00546008) {
              LOCK();
              iVar6 = *piVar9;
              *piVar9 = *piVar9 + -1;
              UNLOCK();
              if (iVar6 + -1 < 1) {
                thunk_FUN_0050e3a0(piVar9);
              }
            }
          }
          uVar16 = iVar14 + 1;
          puVar4 = (undefined *)(ulonglong)uVar16;
        } while ((int)uVar16 < 0x20);
        (**(code **)(*plStack_328 + 0x10))();
        iVar13 = iStackX_20;
      }
    }
    else {
      puVar4 = (undefined *)0x0;
      do {
        uIDNewItem = puVar4;
        FUN_0050daf0((ulonglong *)&plStack_288,0,0x118);
        lpNewItem = &plStack_288;
        iVar14 = (int)uIDNewItem;
        iVar6 = (**(code **)(*plStack_2b0 + 0x90))();
        if (-1 < iVar6) {
          pplStack_370 = (longlong **)PTR_DAT_00546000;
          if (iStack_280 == 0) {
            pWVar10 = (LPCWSTR)CONCAT44(uStack_274,uStack_278);
            if (pWVar10 == (LPCWSTR)0x0) {
              iVar6 = 0;
            }
            else {
              iVar6 = lstrlenW(pWVar10);
            }
            uVar7 = FUN_004020b8((longlong *)&pplStack_370,iVar6);
            pplVar2 = pplStack_370;
            if ((int)uVar7 != 0) {
              uIDNewItem = (undefined *)((longlong)iVar6 * 2);
              FUN_0050dc70(pplStack_370,(undefined8 *)pWVar10,(ulonglong)uIDNewItem);
              *(int *)(pplVar2 + -1) = iVar6;
              *(undefined2 *)((longlong)pplVar2 + (longlong)iVar6 * 2) = 0;
            }
            (*pIStack_358->lpVtbl->Free)(pIStack_358,(void *)CONCAT44(uStack_274,uStack_278));
          }
          else {
            if (iStack_280 == 1) {
              puVar11 = (uint *)((ulonglong)uStack_278 + *param_1);
            }
            else {
              if (iStack_280 != 2) goto LAB_00469452;
              puVar11 = &uStack_278;
            }
            FUN_004033e8((longlong *)&pplStack_370,(LPCSTR)puVar11);
          }
LAB_00469452:
          pplVar2 = pplStack_370;
          if (2 < *(int *)(pplStack_370 + -1)) {
            FUN_00402154(&DAT_005515f0 + iVar13,(longlong *)&pplStack_370);
            if (iVar13 == 0) {
              FUN_0040331c((longlong *)&puStack_308," ");
              puVar12 = puStack_308;
              puStack_2b8 = (undefined8 *)PTR_DAT_00546000;
              FUN_00402204((longlong *)&puStack_2b8,*(int *)(pplVar2 + -1),pplVar2,
                           *(int *)(puStack_308 + -1),puStack_308);
              puVar3 = puStack_2b8;
              FUN_004022a4(&lStack_368,*(int *)(puStack_2b8 + -1),puStack_2b8);
              piVar9 = (int *)((longlong)puVar3 + -0xc);
              if (piVar9 != (int *)PTR_DAT_00546008) {
                LOCK();
                iVar6 = *piVar9;
                *piVar9 = *piVar9 + -1;
                UNLOCK();
                puVar12 = puStack_308;
                if (iVar6 + -1 < 1) {
                  thunk_FUN_0050e3a0(piVar9);
                  puVar12 = puStack_308;
                }
              }
              piVar9 = (int *)((longlong)puVar12 + -0xc);
              if (piVar9 != (int *)PTR_DAT_00546008) {
                LOCK();
                iVar6 = *piVar9;
                *piVar9 = *piVar9 + -1;
                UNLOCK();
                if (iVar6 + -1 < 1) {
                  thunk_FUN_0050e3a0(piVar9);
                  goto LAB_0046952c;
                }
              }
            }
            else {
LAB_0046952c:
              if (iVar13 == 1) {
                FUN_0040329c((longlong *)&puStack_2a8,&lStack_368);
                FUN_0040331c((longlong *)&puStack_300," ");
                puStack_360 = (undefined8 *)PTR_DAT_00546000;
                FUN_00402204((longlong *)&puStack_360,*(int *)(pplVar2 + -1),pplStack_370,
                             *(int *)(puStack_300 + -1),puStack_300);
                puVar12 = puStack_360;
                puStack_310 = PTR_DAT_00546000;
                FUN_00402204((longlong *)&puStack_310,*(int *)(puStack_360 + -1),puStack_360,
                             *(int *)(puStack_2a8 + -1),puStack_2a8);
                FUN_00402154(&lStack_368,(longlong *)&puStack_310);
                piVar9 = (int *)(puStack_310 + -0xc);
                if (piVar9 != (int *)PTR_DAT_00546008) {
                  LOCK();
                  iVar6 = *piVar9;
                  *piVar9 = *piVar9 + -1;
                  UNLOCK();
                  puVar12 = puStack_360;
                  if (iVar6 + -1 < 1) {
                    thunk_FUN_0050e3a0(piVar9);
                    puVar12 = puStack_360;
                  }
                }
                piVar9 = (int *)((longlong)puVar12 + -0xc);
                if (piVar9 != (int *)PTR_DAT_00546008) {
                  LOCK();
                  iVar6 = *piVar9;
                  *piVar9 = *piVar9 + -1;
                  UNLOCK();
                  if (iVar6 + -1 < 1) {
                    thunk_FUN_0050e3a0(piVar9);
                  }
                }
                piVar9 = (int *)((longlong)puStack_300 + -0xc);
                if (piVar9 != (int *)PTR_DAT_00546008) {
                  LOCK();
                  iVar6 = *piVar9;
                  *piVar9 = *piVar9 + -1;
                  UNLOCK();
                  if (iVar6 + -1 < 1) {
                    thunk_FUN_0050e3a0(piVar9);
                  }
                }
                piVar9 = (int *)((longlong)puStack_2a8 + -0xc);
                if (piVar9 != (int *)PTR_DAT_00546008) {
                  LOCK();
                  iVar6 = *piVar9;
                  *piVar9 = *piVar9 + -1;
                  UNLOCK();
                  if (iVar6 + -1 < 1) {
                    thunk_FUN_0050e3a0(piVar9);
                  }
                }
              }
            }
            uIDNewItem = (undefined *)(longlong)(DAT_00546c74 + iVar13);
            lpNewItem = pplStack_370;
            AppendMenuW(param_2,0,(UINT_PTR)uIDNewItem,(LPCWSTR)pplStack_370);
            iVar13 = iVar13 + 1;
          }
          piVar9 = (int *)((longlong)pplStack_370 + -0xc);
          if (piVar9 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar6 = *piVar9;
            *piVar9 = *piVar9 + -1;
            UNLOCK();
            if (iVar6 + -1 < 1) {
              thunk_FUN_0050e3a0(piVar9);
            }
          }
        }
        uVar16 = iVar14 + 1;
        puVar4 = (undefined *)(ulonglong)uVar16;
      } while ((int)uVar16 < 0x20);
    }
    if (pIStack_358 != (IMalloc *)0x0) {
      (*pIStack_358->lpVtbl->Release)(pIStack_358);
    }
    if (*(int *)(lStack_368 + -8) != 0) {
      FUN_00402154(&DAT_005515f0 + iVar13,&lStack_368);
      lpNewItem = (longlong **)(&DAT_005515f0)[iVar13];
      uIDNewItem = (undefined *)(longlong)(DAT_00546c74 + iVar13);
      AppendMenuW(param_2,0,(UINT_PTR)uIDNewItem,(LPCWSTR)lpNewItem);
      iVar13 = iVar13 + 1;
    }
    hMenu = CreatePopupMenu();
    FUN_004a3f04((ulonglong *)&puStack_338,0xb,uIDNewItem,(ulonglong)lpNewItem);
    puVar8 = FUN_004a3f04(&uStack_2e0,0xb,uIDNewItem,(ulonglong)lpNewItem);
    AppendMenuW(param_2,0x10,(ulonglong)hMenu & 0xffffffff,(LPCWSTR)*puVar8);
    piVar9 = (int *)(uStack_2e0 - 0xc);
    if (piVar9 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar6 = *piVar9;
      *piVar9 = *piVar9 + -1;
      UNLOCK();
      if (iVar6 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(uStack_2e0 - 0xc));
      }
    }
    iVar6 = iVar13 * 2;
    iVar14 = 0;
    if (iVar13 < iVar6) {
      plVar15 = &DAT_005515f0 + iVar13;
      do {
        FUN_0040329c((longlong *)&pWStack_2c0,&DAT_005515f0 + iVar14);
        pWVar10 = pWStack_2c0;
        AppendMenuW(hMenu,0,(longlong)(iVar13 + DAT_00546c74),pWStack_2c0);
        FUN_0040331c((longlong *)&puStack_2d0," ");
        puStack_350 = (undefined8 *)PTR_DAT_00546000;
        FUN_00402204((longlong *)&puStack_350,*(int *)(puStack_338 + -1),puStack_338,
                     *(int *)(puStack_2d0 + -1),puStack_2d0);
        puVar12 = puStack_350;
        puStack_318 = PTR_DAT_00546000;
        FUN_00402204((longlong *)&puStack_318,*(int *)(puStack_350 + -1),puStack_350,
                     *(int *)(pWVar10 + -4),(undefined8 *)pWVar10);
        FUN_00402154(plVar15,(longlong *)&puStack_318);
        piVar9 = (int *)(puStack_318 + -0xc);
        if (piVar9 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar1 = *piVar9;
          *piVar9 = *piVar9 + -1;
          UNLOCK();
          puVar12 = puStack_350;
          if (iVar1 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar9);
            puVar12 = puStack_350;
          }
        }
        piVar9 = (int *)((longlong)puVar12 + -0xc);
        if (piVar9 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar1 = *piVar9;
          *piVar9 = *piVar9 + -1;
          UNLOCK();
          if (iVar1 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar9);
          }
        }
        piVar9 = (int *)((longlong)puStack_2d0 + -0xc);
        if (piVar9 != (int *)PTR_DAT_00546008) {
          LOCK();
          iVar1 = *piVar9;
          *piVar9 = *piVar9 + -1;
          UNLOCK();
          if (iVar1 + -1 < 1) {
            thunk_FUN_0050e3a0(piVar9);
          }
        }
        iVar14 = iVar14 + 1;
        pWVar10 = pWStack_2c0 + -6;
        if (pWVar10 != (LPCWSTR)PTR_DAT_00546008) {
          LOCK();
          iVar1 = *(int *)pWVar10;
          *(int *)pWVar10 = *(int *)pWVar10 + -1;
          UNLOCK();
          if (iVar1 + -1 < 1) {
            thunk_FUN_0050e3a0(pWVar10);
          }
        }
        iVar13 = iVar13 + 1;
        plVar15 = plVar15 + 1;
      } while (iVar13 < iVar6);
    }
    piVar9 = (int *)((longlong)puStack_338 + -0xc);
    if (piVar9 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar13 = *piVar9;
      *piVar9 = *piVar9 + -1;
      UNLOCK();
      if (iVar13 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)((longlong)puStack_338 + -0xc));
      }
    }
    piVar9 = (int *)(lStack_368 + -0xc);
    if (piVar9 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar13 = *piVar9;
      *piVar9 = *piVar9 + -1;
      UNLOCK();
      if (iVar13 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar9);
      }
    }
    if (plStack_340 != (longlong *)0x0) {
      (**(code **)(*plStack_340 + 0x10))();
      plStack_340 = (longlong *)0x0;
    }
  }
  FUN_004bea74(auStack_2a0);
  return;
}


