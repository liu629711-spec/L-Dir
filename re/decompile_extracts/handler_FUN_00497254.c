// ==== FUN_00497254 @ 00497254 ====

undefined8 FUN_00497254(IDropTargetVtbl *param_1,int param_2)

{
  WCHAR WVar1;
  WCHAR WVar2;
  bool bVar3;
  int iVar4;
  undefined8 uVar5;
  longlong lVar6;
  LPWSTR lpString;
  LPCWSTR pWVar7;
  bool bVar8;
  int local_res10 [2];
  LPCWSTR local_res18 [2];
  undefined **local_58;
  longlong local_50;
  undefined **local_48;
  undefined8 local_40;
  
  if (DAT_005542a8 == 0) {
    DAT_005542a8 = 1;
    local_res10[0] = param_2;
    lVar6 = FUN_00496b4c((longlong)&param_1[0xe].DragOver,local_res10);
    FUN_004045dc(&local_50,(longlong *)(-(ulonglong)(lVar6 != 0) & lVar6 + 8U));
    bVar3 = false;
    local_58 = &PTR_FUN_00527d50;
    bVar8 = false;
    if ((((*(int *)((longlong)&param_1[10].DragLeave + 4) == *(int *)(lVar6 + 0x24)) &&
         (bVar8 = bVar3, *(int *)&param_1[10].DragOver == *(int *)(lVar6 + 0x18))) &&
        (*(int *)((longlong)&param_1[10].DragOver + 4) == *(int *)(lVar6 + 0x1c))) &&
       (*(int *)&param_1[10].DragLeave == *(int *)(lVar6 + 0x20))) {
      FUN_0040331c((longlong *)local_res18,"");
      lpString = (LPWSTR)FUN_00402340((longlong *)local_res18,0x800);
      GetWindowTextW((HWND)param_1[6].DragOver,lpString,0x800);
      FUN_00402010((longlong *)local_res18);
      iVar4 = lstrlenW(local_res18[0]);
      *(int *)(local_res18[0] + -4) = iVar4;
      local_res18[0][iVar4] = L'\0';
      pWVar7 = local_res18[0];
      do {
        WVar1 = *pWVar7;
        WVar2 = *(WCHAR *)((longlong)pWVar7 +
                          (*(longlong *)(lVar6 + 0x28) - (longlong)local_res18[0]));
        if (WVar1 != WVar2) break;
        pWVar7 = pWVar7 + 1;
      } while (WVar2 != L'\0');
      bVar8 = WVar1 == WVar2;
      local_res18[0] = local_res18[0] + -6;
      if (local_res18[0] != (LPCWSTR)PTR_DAT_00546008) {
        LOCK();
        iVar4 = *(int *)local_res18[0];
        *(int *)local_res18[0] = *(int *)local_res18[0] + -1;
        UNLOCK();
        param_2 = local_res10[0];
        if (iVar4 + -1 < 1) {
          thunk_FUN_0050e3a0(local_res18[0]);
          param_2 = local_res10[0];
        }
      }
    }
    *(int *)((longlong)&param_1[0xb].DragEnter + 4) = param_2;
    if (DAT_00547bb0 == 0) {
      param_1[0xb].DragOver = (_func_6379 *)0x0;
    }
    else {
      param_1[0xb].DragOver = (_func_6379 *)(lVar6 + 0x40);
    }
    *(int *)((longlong)&param_1[6].DragEnter + 4) = param_2;
    FUN_004045dc(&local_40,&local_50);
    local_48 = &PTR_FUN_00527d50;
    FUN_0048dffc(param_1,&local_48,1,0);
    *(int *)&param_1[10].DragEnter = param_2;
    param_1[0xb].DragOver = (_func_6379 *)0x0;
    *(undefined4 *)((longlong)&param_1[0xb].DragEnter + 4) = 0xffffffff;
    if ((DAT_00547bac != 0) && (bVar8)) {
      FUN_0048073c((longlong)param_1,lVar6 + 0x30);
    }
    FUN_00489b4c((longlong)param_1,0);
    DAT_005542a8 = 0;
    FUN_004efeb4(&local_58);
    uVar5 = 1;
  }
  else {
    uVar5 = 0;
  }
  return uVar5;
}


