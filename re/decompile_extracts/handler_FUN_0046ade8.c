// ==== FUN_0046ade8 @ 0046ade8 ====

undefined8 FUN_0046ade8(longlong param_1,undefined8 param_2,undefined8 param_3,longlong *param_4)

{
  uint uVar1;
  int iVar2;
  undefined8 *puVar3;
  wchar_t *pwVar4;
  longlong lVar5;
  LPCWSTR pWVar6;
  LPCWSTR local_res20;
  LPCWSTR local_a8;
  undefined **local_a0;
  undefined8 local_98;
  undefined8 local_90;
  longlong local_88;
  undefined **local_80;
  undefined8 local_78;
  undefined8 local_70;
  longlong local_68;
  WINDOWPLACEMENT local_60;
  
  if ((param_4 != (longlong *)0x0) && (*param_4 == 0x2774)) {
    FUN_00407e50((longlong *)&local_res20,(LPCWSTR)param_4[2]);
    uVar1 = *(uint *)(param_4 + 1);
    puVar3 = (undefined8 *)FUN_00402340((longlong *)&local_res20,uVar1);
    FUN_0050dc70(puVar3,(undefined8 *)param_4[2],(ulonglong)uVar1);
    FUN_00402010((longlong *)&local_res20);
    pWVar6 = local_res20;
    iVar2 = lstrlenW(local_res20);
    *(int *)(pWVar6 + -4) = iVar2;
    pWVar6[iVar2] = L'\0';
    if ((-1 < *(int *)(pWVar6 + -4)) &&
       ((pwVar4 = wcsstr(pWVar6,L"/e"), pwVar4 != (wchar_t *)0x0 &&
        ((int)((longlong)pwVar4 - (longlong)pWVar6 >> 1) == 0)))) {
      FUN_0050daf0((ulonglong *)&local_60,0,0x2c);
      local_60.flags = 2;
      local_60.length = 0x2c;
      GetWindowPlacement(*(HWND *)(param_1 + 8),&local_60);
      if (local_60.showCmd == 2) {
        SendMessageW(*(HWND *)(param_1 + 8),0x112,0xf120,0);
      }
      else if (*(int *)(param_1 + 0x4e24) == 0) {
        SetWindowPos(*(HWND *)(param_1 + 8),(HWND)0xffffffffffffffff,0,0,0,0,3);
        SetWindowPos(*(HWND *)(param_1 + 8),(HWND)0xfffffffffffffffe,0,0,0,0,3);
        SetWindowPos(*(HWND *)(param_1 + 8),(HWND)0x0,0,0,0,0,0x43);
      }
      FUN_004033e8(&DAT_00551030,"");
      FUN_004033e8((longlong *)&DAT_00551038,"");
      FUN_0040329c((longlong *)&local_a8,(longlong *)&local_res20);
      FUN_00462130((longlong *)&local_a8);
      lVar5 = FUN_00456560(param_1);
      if ((lVar5 != 0) && (*(int *)(DAT_00551030 + -8) != 0)) {
        if (*(int *)(DAT_00551038 + -4) == 0) {
          FUN_004eff4c(&local_90,local_a8);
          FUN_004045dc(&local_98,&local_88);
          local_a0 = &PTR_FUN_00527d50;
          FUN_004d02a4(lVar5,&local_a0,-1);
          puVar3 = &local_90;
        }
        else {
          FUN_004eff4c(&local_70,DAT_00551038);
          FUN_004045dc(&local_78,&local_68);
          local_80 = &PTR_FUN_00527d50;
          FUN_004d01b8(lVar5,&local_80,-1);
          puVar3 = &local_70;
        }
        FUN_004efeb4(puVar3);
      }
      SetTimer(*(HWND *)(param_1 + 8),0x186ce,300,(TIMERPROC)0x0);
      local_a8 = local_a8 + -6;
      if (local_a8 != (LPCWSTR)PTR_DAT_00546008) {
        LOCK();
        iVar2 = *(int *)local_a8;
        *(int *)local_a8 = *(int *)local_a8 + -1;
        UNLOCK();
        pWVar6 = local_res20;
        if (iVar2 + -1 < 1) {
          thunk_FUN_0050e3a0(local_a8);
          pWVar6 = local_res20;
        }
      }
      pWVar6 = pWVar6 + -6;
      if (pWVar6 != (LPCWSTR)PTR_DAT_00546008) {
        LOCK();
        iVar2 = *(int *)pWVar6;
        *(int *)pWVar6 = *(int *)pWVar6 + -1;
        UNLOCK();
        if (iVar2 + -1 < 1) {
          thunk_FUN_0050e3a0(pWVar6);
        }
      }
      return 0x2774;
    }
    pWVar6 = pWVar6 + -6;
    if (pWVar6 != (LPCWSTR)PTR_DAT_00546008) {
      LOCK();
      iVar2 = *(int *)pWVar6;
      *(int *)pWVar6 = *(int *)pWVar6 + -1;
      UNLOCK();
      if (iVar2 + -1 < 1) {
        thunk_FUN_0050e3a0(pWVar6);
      }
    }
  }
  return 0;
}


