// ==== FUN_0049747c @ 0049747c ====

longlong FUN_0049747c(IDropTargetVtbl *param_1,undefined8 param_2,ulonglong param_3,longlong param_4
                     ,undefined4 *param_5)

{
  _func_6376 *p_Var1;
  undefined4 *puVar2;
  char cVar3;
  int iVar4;
  int iVar5;
  longlong *plVar6;
  undefined8 uVar7;
  undefined8 *puVar8;
  HWND pHVar9;
  int *piVar10;
  longlong local_res18;
  ushort *local_res20;
  undefined8 local_88 [2];
  undefined8 local_78 [2];
  undefined1 local_68 [12];
  undefined4 local_5c;
  undefined4 local_58;
  
  puVar2 = param_5;
  *param_5 = 0;
  if (param_3 < 0xa773) {
    if (param_3 == 0xa772) {
      uVar7 = FUN_00481604((longlong)param_1);
      if ((int)uVar7 != 0) {
        local_58 = 2;
        local_5c = 0;
        pHVar9 = (HWND)FUN_004805d0((longlong)param_1);
        SendMessageW(pHVar9,0x102b,0xffffffffffffffff,(LPARAM)local_68);
        FUN_00486350((longlong)param_1,L"NewFolder");
      }
    }
    else if (param_3 == 0x802d) {
      FUN_0048cf6c((longlong)param_1);
    }
    else if (param_3 == 0x805e) {
      p_Var1 = param_1[7].AddRef;
      *(undefined4 *)&param_1[10].DragOver = *(undefined4 *)(p_Var1 + 0x350);
      *(undefined4 *)((longlong)&param_1[10].DragOver + 4) = *(undefined4 *)(p_Var1 + 0x354);
      *(undefined4 *)&param_1[10].DragLeave = *(undefined4 *)(p_Var1 + 0x358);
      *(undefined4 *)((longlong)&param_1[10].DragLeave + 4) = *(undefined4 *)(p_Var1 + 0x35c);
      *(undefined4 *)&param_1[10].Drop = *(undefined4 *)(p_Var1 + 0x360);
      FUN_00402154((longlong *)(param_1 + 0xb),(longlong *)(p_Var1 + 0x368));
      p_Var1 = param_1[7].AddRef;
      *(uint *)((longlong)&param_1[10].DragLeave + 4) =
           (uint)(*(int *)((longlong)&param_1[10].DragLeave + 4) == 0);
      *(undefined4 *)(p_Var1 + 0x350) = *(undefined4 *)&param_1[10].DragOver;
      *(undefined4 *)(p_Var1 + 0x354) = *(undefined4 *)((longlong)&param_1[10].DragOver + 4);
      *(undefined4 *)(p_Var1 + 0x358) = *(undefined4 *)&param_1[10].DragLeave;
      *(undefined4 *)(p_Var1 + 0x35c) = *(undefined4 *)((longlong)&param_1[10].DragLeave + 4);
      *(undefined4 *)(p_Var1 + 0x360) = *(undefined4 *)&param_1[10].Drop;
      FUN_00402154((longlong *)(p_Var1 + 0x368),(longlong *)(param_1 + 0xb));
      *puVar2 = 1;
      FUN_0047d8e4(*(LPCWSTR *)(param_1[7].AddRef + 0x78),L"show_filter",
                   (ulonglong)*(uint *)((longlong)&param_1[10].DragLeave + 4),(LPCWSTR)0x0);
      FUN_0048cb04((longlong)param_1,(ulonglong)*(uint *)((longlong)&param_1[10].DragLeave + 4),1,0)
      ;
      SendMessageW((HWND)param_1[6].DragOver,0xb1,0,-1);
    }
    else {
      if (param_3 == 0x805f) {
        pHVar9 = FindWindowExW((HWND)param_1[2].AddRef,(HWND)0x0,L"SysListView32",(LPCWSTR)0x0);
        if (pHVar9 != (HWND)0x0) {
          SetFocus(pHVar9);
        }
        iVar4 = 1;
      }
      else {
        if (param_3 != 0x8062) {
          if (param_3 < 0xa76c) {
            return 0;
          }
          if (param_3 < 0xa76e) {
            iVar4 = 1;
            *param_5 = 1;
            if (param_3 == 0xa76d) {
              iVar5 = *(int *)&param_1[10].DragEnter;
              if (*(int *)&param_1[0xf].AddRef <= iVar5) {
                return 0;
              }
            }
            else {
              if (param_3 != 0xa76c) {
                return 0;
              }
              iVar5 = *(int *)&param_1[10].DragEnter;
              if (iVar5 < 1) {
                return 0;
              }
              iVar4 = -1;
            }
            FUN_00497254(param_1,iVar5 + iVar4);
            return 0;
          }
          if (param_3 == 0xa770) {
            p_Var1 = param_1[7].AddRef;
            if (((p_Var1 != (_func_6376 *)0x0) && (DAT_00555270 != 0)) &&
               (*(int *)(p_Var1 + 0x6ac) != 0)) {
              FUN_004f51c8((longlong)(p_Var1 + 0x470));
            }
            FUN_0048a144(param_1);
            return 0;
          }
          if (param_3 != 0xa771) {
            return 0;
          }
          if (DAT_005542ac != 0) {
            return 0;
          }
          DAT_005542ac = 1;
          local_res20 = (ushort *)0x0;
          param_5 = (undefined4 *)0x0;
          cVar3 = FUN_004be9b0(&param_1[1].DragLeave,(LPCITEMIDLIST)param_1[0xd].Drop,
                               (longlong *)&local_res20,&param_5);
          if (cVar3 != '\0') {
            plVar6 = FUN_004ef70c((longlong)&param_1[0xd].DragLeave,&local_res18);
            FUN_004eff4c(local_88,(LPCWSTR)*plVar6);
            piVar10 = (int *)(local_res18 + -0xc);
            if (piVar10 != (int *)PTR_DAT_00546008) {
              LOCK();
              iVar4 = *piVar10;
              *piVar10 = *piVar10 + -1;
              UNLOCK();
              if (iVar4 + -1 < 1) {
                thunk_FUN_0050e3a0((LPVOID)(local_res18 + -0xc));
              }
            }
            param_5 = (undefined4 *)FUN_004ef0c4((longlong)local_88);
            uVar7 = FUN_004eef90((longlong)&param_1[0xd].DragLeave);
            if ((int)uVar7 != 0) {
              DAT_005542ac = 0;
              FUN_004efeb4(local_88);
              return 0;
            }
            puVar8 = FUN_004efe84(local_78,local_res20);
            iVar4 = FUN_0048dffc(param_1,puVar8,0x2001,1);
            if (iVar4 != 0) {
              FUN_00480b24((longlong)param_1,param_5);
            }
            DAT_005542ac = 0;
            FUN_004efeb4(local_88);
          }
          *puVar2 = 1;
          return 1;
        }
        pHVar9 = FindWindowExW((HWND)param_1[2].AddRef,(HWND)0x0,L"SysListView32",(LPCWSTR)0x0);
        if (pHVar9 != (HWND)0x0) {
          SetFocus(pHVar9);
        }
        iVar4 = 0;
      }
      FUN_004828e0((longlong)param_1,iVar4);
    }
  }
  else if (param_3 == 0xb37b) {
    FUN_0048286c((longlong)param_1);
  }
  else if (param_3 == 0xe120) {
    if ((param_4 == *(longlong *)(param_1[7].AddRef + 0x388)) &&
       (pHVar9 = FindWindowExW((HWND)param_1[2].AddRef,(HWND)0x0,L"SysListView32",(LPCWSTR)0x0),
       pHVar9 != (HWND)0x0)) {
      SetFocus(pHVar9);
    }
    PostMessageW((HWND)param_1->AddRef,0x100,0x2e,0);
  }
  else {
    if (param_3 == 0xe122) {
      iVar4 = 0xe122;
    }
    else if (param_3 == 0xe123) {
      if ((param_4 == *(longlong *)(param_1[7].AddRef + 0x388)) &&
         (pHVar9 = FindWindowExW((HWND)param_1[2].AddRef,(HWND)0x0,L"SysListView32",(LPCWSTR)0x0),
         pHVar9 != (HWND)0x0)) {
        SetFocus(pHVar9);
      }
      iVar4 = 0xe123;
    }
    else {
      if (param_3 != 0xe125) {
        if (param_3 != 0x1b208) {
          return 0;
        }
        *param_5 = 1;
        uVar7 = FUN_0048c6d0((longlong)param_1);
        return (longlong)(int)uVar7;
      }
      iVar4 = 0xe125;
    }
    FUN_0048b468((longlong)param_1,iVar4);
  }
  return 0;
}


